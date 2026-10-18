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
pub struct StudentSchoolInfoListing {
    #[serde(rename = "school")]
    pub school: String,
    #[serde(rename = "principal")]
    pub principal: String,
    #[serde(rename = "schoolAddress")]
    pub school_address: String,
    #[serde(rename = "schoolAddress2")]
    pub school_address2: String,
    #[serde(rename = "schoolCity")]
    pub school_city: String,
    #[serde(rename = "schoolState")]
    pub school_state: String,
    #[serde(rename = "schoolZip")]
    pub school_zip: String,
    #[serde(rename = "phone")]
    pub phone: String,
    #[serde(rename = "phone2")]
    pub phone2: String,
    #[serde(rename = "URL")]
    pub url: String,
    #[serde(rename = "principalEmail")]
    pub principal_email: String,
    #[serde(rename = "principalGu")]
    pub principal_gu: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "staffLists")]
    pub staff_lists: Vec<Staff>,
}

#[derive(Serialize, Deserialize)]
pub struct Staff {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "eMail")]
    pub email: String,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "phone")]
    pub phone: String,
    #[serde(rename = "extn")]
    pub extn: String,
    #[serde(rename = "staffGU")]
    pub staff_gu: String,
}
