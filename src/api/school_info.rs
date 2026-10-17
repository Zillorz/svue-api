use crate::{
    api::{api_request, ApiError},
    crypto::AuthToken,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

pub async fn school_info(token: &mut AuthToken) -> Result<SchoolInfo, ApiError> {
    let data: SchoolInfoData =
        api_request("GetSchoolInformationData", json!({"childIntID": 0}), token).await?;
    Ok(data.student_school_info_listing.into())
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
            staff: value
                .staff_lists
                .into_iter()
                .map(|x| x.into())
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct StaffInfo {
    name: String,
    job_title: String,
    email: String,
}

impl From<StaffList> for StaffInfo {
    fn from(value: StaffList) -> Self {
        StaffInfo {
            name: value.name,
            job_title: value.title,
            email: value.email,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SchoolInfoData {
    student_school_info_listing: StudentSchoolInfoListing,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentSchoolInfoListing {
    pub school: String,
    pub principal: String,
    pub school_address: String,
    pub school_city: String,
    pub school_state: String,
    pub school_zip: String,
    pub phone: String,
    #[serde(rename = "URL")]
    pub url: String,
    pub principal_email: String,
    #[serde(default)]
    pub staff_lists: Vec<StaffList>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StaffList {
    pub name: String,
    #[serde(rename = "eMail")]
    pub email: String,
    pub title: String,
}
