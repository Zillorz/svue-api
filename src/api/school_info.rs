use crate::{
    api::{api_request, ApiError, ProcessWebServiceRequest},
    crypto::AuthToken,
};
use serde::{Deserialize, Serialize};

pub async fn school_info(token: &mut AuthToken) -> Result<SchoolInfo, ApiError> {
    let result = api_request(
        ProcessWebServiceRequest::ck_default("StudentSchoolInfo".to_string(), String::new(), token),
        token,
    )
    .await?;

    let si: StudentSchoolInfoListing = quick_xml::de::from_str(result.as_str())?;
    Ok(si.into())
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
                .staff_list
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

// Xml structs

#[derive(Serialize, Deserialize)]
pub struct StudentSchoolInfoListing {
    #[serde(rename = "@xmlns:xsd")]
    pub xmlns_xsd: String,
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "@School")]
    pub school: String,
    #[serde(rename = "@Principal")]
    pub principal: String,
    #[serde(rename = "@SchoolAddress")]
    pub school_address: String,
    #[serde(rename = "@SchoolAddress2")]
    pub school_address2: String,
    #[serde(rename = "@SchoolCity")]
    pub school_city: String,
    #[serde(rename = "@SchoolState")]
    pub school_state: String,
    #[serde(rename = "@SchoolZip")]
    pub school_zip: String,
    #[serde(rename = "@Phone")]
    pub phone: String,
    #[serde(rename = "@Phone2")]
    pub phone2: String,
    #[serde(rename = "@URL")]
    pub url: String,
    #[serde(rename = "@PrincipalEmail")]
    pub principal_email: String,
    #[serde(rename = "@PrincipalGu")]
    pub principal_gu: String,
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "StaffLists")]
    pub staff_lists: StaffLists,
}

#[derive(Serialize, Deserialize)]
pub struct StaffLists {
    #[serde(rename = "$text")]
    pub text: Option<String>,
    #[serde(rename = "StaffList")]
    pub staff_list: Vec<StaffList>,
}

#[derive(Serialize, Deserialize)]
pub struct StaffList {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@EMail")]
    pub email: String,
    #[serde(rename = "@Title")]
    pub title: String,
    #[serde(rename = "@Phone")]
    pub phone: String,
    #[serde(rename = "@Extn")]
    pub extn: String,
    #[serde(rename = "@StaffGU")]
    pub staff_gu: String,
}

