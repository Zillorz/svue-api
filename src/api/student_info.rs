use crate::api::{ApiError, DefaultReq, api_req};
use crate::crypto::AuthToken;
use crate::documents::base64;
use serde::{Deserialize, Serialize};

pub async fn both(token: &mut AuthToken) -> Result<(StudentInfo, Vec<u8>), ApiError> {
    let result: OuterStudent = api_req("GetStudentInfoData", DefaultReq::default(), token).await?;

    let mut si: StudentInfo_ = result.inner;
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
    school: String,
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
            school: value.current_school,
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

// Xml structs

#[derive(Serialize, Deserialize)]
pub struct OuterStudent {
    #[serde(rename = "studentInfoDetailXML")]
    inner: StudentInfo_,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentInfo_ {
    #[serde(rename = "type")]
    pub student_info_type: String,
    // #[serde(rename = "@ShowStudentBusAssignmentInfo")]
    // pub show_student_bus_assignment_info: String,
    // #[serde(rename = "@ShowPhysicianAndDentistInfo")]
    // pub show_physician_and_dentist_info: String,
    // #[serde(rename = "@ShowStudentInfo")]
    // pub show_student_info: String,
    // #[serde(rename = "@ShowFrontLineSpedURL")]
    // pub show_front_line_sped_url: String,
    // #[serde(rename = "@ShowFrontLine504URL")]
    // pub show_front_line504_url: String,
    // #[serde(rename = "@ShowFrontLineParentPortalURL")]
    // pub show_front_line_parent_portal_url: String,
    // #[serde(rename = "LockerInfoRecords")]
    // pub locker_info_records: LockerInfoRecords,
    pub formatted_name: String,
    #[serde(rename = "permID")]
    pub perm_id: String,
    pub gender: String,
    pub grade: String,
    pub address: String,
    pub last_name_goes_by: String,
    pub nick_name: String,
    pub birth_date: String,
    #[serde(rename = "eMail")]
    pub email: String,
    pub phone: String,
    pub home_language: String,
    pub current_school: String,
    pub track: String,
    pub home_room_tch: String,
    #[serde(rename = "homeRoomTchEMail")]
    pub home_room_tch_email: String,
    #[serde(rename = "homeRoomTchStaffGU")]
    pub home_room_tch_staff_gu: String,
    #[serde(rename = "orgYearGU")]
    pub org_year_gu: String,
    pub home_room: String,
    pub counselor_name: String,
    pub counselor_email: String,
    #[serde(rename = "counselorStaffGU")]
    pub counselor_staff_gu: String,
    #[serde(deserialize_with = "base64")]
    pub photo: Vec<u8>,
    pub emergency_contacts: Vec<EmergencyContact>,
    pub physician: Physician,
    pub dentist: Dentist,
    // #[serde(rename = "UserDefinedGroupBoxes")]
    // pub user_defined_group_boxes: UserDefinedGroupBoxes,
    // #[serde(rename = "StudentBusAssignments")]
    // pub student_bus_assignments: StudentBusAssignments,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyContact {
    pub name: String,
    pub relationship: String,
    pub home_phone: String,
    pub work_phone: String,
    pub other_phone: String,
    pub mobile_phone: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Physician {
    pub name: String,
    pub hospital: String,
    pub phone: String,
    pub extn: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dentist {
    pub name: String,
    pub office: String,
    pub phone: String,
    pub extn: String,
}
