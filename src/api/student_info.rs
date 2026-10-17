use crate::api::{api_request, ApiError};
use crate::crypto::AuthToken;
use crate::documents::base64;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub async fn both(token: &mut AuthToken) -> Result<(StudentInfo, Vec<u8>), ApiError> {
    let data: StudentInfoData =
        api_request("GetStudentInfoData", json!({"childIntID": 0}), token).await?;

    let mut si = data.student_info_detail_xml;
    let photo = std::mem::take(&mut si.photo);
    Ok((si.into(), photo))
}

pub async fn student_info(token: &mut AuthToken) -> Result<StudentInfo, ApiError> {
    Ok(both(token).await?.0)
}

pub async fn photo(token: &mut AuthToken) -> Result<Vec<u8>, ApiError> {
    Ok(both(token).await?.1)
}

#[derive(Serialize, Deserialize)]
pub struct StudentInfo {
    pub name: String,
    pub id: String,
    gender: String,
    grade: String,
    address: String,
    birth_date: String,
    email: String,
    phone_number: String,
    emergency_contacts: Vec<Contact>,
    physician: Doctor,
    dentist: Doctor,
    school: String
}

impl From<StudentInfo_> for StudentInfo {
    fn from(value: StudentInfo_) -> Self {
        StudentInfo {
            name: value.formatted_name,
            id: value.perm_id,
            gender: value.gender,
            grade: value.grade,
            address: value.address.replace("<br>", "\n").to_string(),
            birth_date: value.birth_date,
            email: value.email,
            phone_number: value.phone,
            emergency_contacts: value
                .emergency_contacts
                .into_iter()
                .map(|x| x.into())
                .collect(),
            physician: value.physician.into(),
            dentist: value.dentist.into(),
            school: value.current_school
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Contact {
    name: String,
    relation: String,
    phone_numbers: Vec<String>,
}

impl From<EmergencyContact> for Contact {
    fn from(value: EmergencyContact) -> Self {
        let mut numbers = Vec::new();

        if !value.mobile_phone.is_empty() {
            numbers.push(value.mobile_phone);
        }
        if !value.home_phone.is_empty() {
            numbers.push(value.home_phone);
        }
        if !value.work_phone.is_empty() {
            numbers.push(value.work_phone);
        }
        if !value.other_phone.is_empty() {
            numbers.push(value.other_phone);
        }

        Contact {
            name: value.name,
            relation: value.relationship,
            phone_numbers: numbers,
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Doctor {
    name: String,
    workplace: String,
    phone_number: String,
}

impl From<Physician> for Doctor {
    fn from(value: Physician) -> Self {
        Doctor {
            name: value.name,
            workplace: value.hospital,
            phone_number: value.phone,
        }
    }
}

impl From<Dentist> for Doctor {
    fn from(value: Dentist) -> Self {
        Doctor {
            name: value.name,
            workplace: value.office,
            phone_number: value.phone,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudentInfoData {
    #[serde(rename = "studentInfoDetailXML")]
    student_info_detail_xml: StudentInfo_,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentInfo_ {
    pub formatted_name: String,
    #[serde(rename = "permID")]
    pub perm_id: String,
    pub gender: String,
    pub grade: String,
    pub address: String,
    pub birth_date: String,
    #[serde(rename = "eMail")]
    pub email: String,
    pub phone: String,
    pub current_school: String,
    #[serde(deserialize_with = "base64")]
    pub photo: Vec<u8>,
    #[serde(default)]
    pub emergency_contacts: Vec<EmergencyContact>,
    pub physician: Physician,
    pub dentist: Dentist,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyContact {
    pub name: String,
    pub relationship: String,
    pub home_phone: String,
    pub work_phone: String,
    pub other_phone: String,
    pub mobile_phone: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Physician {
    pub name: String,
    pub hospital: String,
    pub phone: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dentist {
    pub name: String,
    pub office: String,
    pub phone: String,
}
