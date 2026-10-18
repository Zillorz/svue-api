use crate::{
    api::{ApiError, DefaultReq, api_req},
    crypto::AuthToken,
};
use serde::{Deserialize, Serialize};

pub async fn school_info(token: &mut AuthToken) -> Result<SchoolInfo, ApiError> {
    let result: OuterSchoolInfo =
        api_req("GetSchoolInformationData", DefaultReq::default(), token).await?;

    Ok(result.inner.into())
}

#[derive(Serialize, Deserialize)]
pub struct SchoolInfo {
    name: String,
    principal: String,
    principal_email: String,
    address: String,
    city: String,
    state: String,
    zip_code: String,
    phone_number: String,
    website: String,
    staff: Vec<StaffInfo>,
}

impl From<StudentSchoolInfoListing> for SchoolInfo {
    fn from(value: StudentSchoolInfoListing) -> Self {
        SchoolInfo {
            name: value.school,
            principal: value.principal,
            principal_email: value.principal_email,
            address: value.school_address,
            city: value.school_city,
            state: value.school_state,
            zip_code: value.school_zip,
            phone_number: value.phone,
            website: value.url,
            staff: value.staff_lists.into_iter().map(|x| x.into()).collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StaffInfo {
    name: String,
    job_title: String,
    email: String,
}

impl From<Staff> for StaffInfo {
    fn from(value: Staff) -> Self {
        StaffInfo {
            name: value.name,
            job_title: value.title,
            email: value.email,
        }
    }
}

// Xml structs
#[derive(Serialize, Deserialize)]
pub struct OuterSchoolInfo {
    #[serde(rename = "studentSchoolInfoListing")]
    pub inner: StudentSchoolInfoListing,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentSchoolInfoListing {
    pub school: String,
    pub principal: String,
    pub school_address: String,
    pub school_address2: String,
    pub school_city: String,
    pub school_state: String,
    pub school_zip: String,
    pub phone: String,
    pub phone2: String,
    #[serde(rename = "URL")]
    pub url: String,
    pub principal_email: String,
    pub principal_gu: String,
    pub staff_lists: Vec<Staff>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Staff {
    pub name: String,
    #[serde(rename = "eMail")]
    pub email: String,
    pub title: String,
    pub phone: String,
    pub extn: String,
    #[serde(rename = "staffGU")]
    pub staff_gu: String,
}
