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
    #[serde(rename = "formattedName")]
    pub formatted_name: String,
    #[serde(rename = "permID")]
    pub perm_id: String,
    #[serde(rename = "gender")]
    pub gender: String,
    #[serde(rename = "grade")]
    pub grade: String,
    #[serde(rename = "address")]
    pub address: String,
    #[serde(rename = "lastNameGoesBy")]
    pub last_name_goes_by: String,
    #[serde(rename = "nickName")]
    pub nick_name: String,
    #[serde(rename = "birthDate")]
    pub birth_date: String,
    #[serde(rename = "eMail")]
    pub email: String,
    #[serde(rename = "phone")]
    pub phone: String,
    #[serde(rename = "homeLanguage")]
    pub home_language: String,
    #[serde(rename = "currentSchool")]
    pub current_school: String,
    #[serde(rename = "track")]
    pub track: String,
    #[serde(rename = "homeRoomTch")]
    pub home_room_tch: String,
    #[serde(rename = "homeRoomTchEMail")]
    pub home_room_tch_email: String,
    #[serde(rename = "homeRoomTchStaffGU")]
    pub home_room_tch_staff_gu: String,
    #[serde(rename = "orgYearGU")]
    pub org_year_gu: String,
    #[serde(rename = "homeRoom")]
    pub home_room: String,
    #[serde(rename = "counselorName")]
    pub counselor_name: String,
    #[serde(rename = "counselorEmail")]
    pub counselor_email: String,
    #[serde(rename = "counselorStaffGU")]
    pub counselor_staff_gu: String,
    #[serde(rename = "photo")]
    #[serde(deserialize_with = "base64")]
    pub photo: Vec<u8>,
    #[serde(rename = "emergencyContacts")]
    pub emergency_contacts: Vec<EmergencyContact>,
    #[serde(rename = "physician")]
    pub physician: Physician,
    #[serde(rename = "dentist")]
    pub dentist: Dentist,
    // #[serde(rename = "UserDefinedGroupBoxes")]
    // pub user_defined_group_boxes: UserDefinedGroupBoxes,
    // #[serde(rename = "StudentBusAssignments")]
    // pub student_bus_assignments: StudentBusAssignments,
}

#[derive(Serialize, Deserialize)]
pub struct EmergencyContact {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "relationship")]
    pub relationship: String,
    #[serde(rename = "homePhone")]
    pub home_phone: String,
    #[serde(rename = "workPhone")]
    pub work_phone: String,
    #[serde(rename = "otherPhone")]
    pub other_phone: String,
    #[serde(rename = "mobilePhone")]
    pub mobile_phone: String,
}

#[derive(Serialize, Deserialize)]
pub struct Physician {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "hospital")]
    pub hospital: String,
    #[serde(rename = "phone")]
    pub phone: String,
    #[serde(rename = "extn")]
    pub extn: String,
}

#[derive(Serialize, Deserialize)]
pub struct Dentist {
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "office")]
    pub office: String,
    #[serde(rename = "phone")]
    pub phone: String,
    #[serde(rename = "extn")]
    pub extn: String,
}
