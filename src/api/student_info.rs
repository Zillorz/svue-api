use crate::api::{api_request, ApiError, ProcessWebServiceRequest};
use crate::crypto::AuthToken;
use crate::documents::base64;
use serde::{Deserialize, Serialize};

pub async fn both(token: &mut AuthToken) -> Result<(StudentInfo, Vec<u8>), ApiError> {
    let result = api_request(
        ProcessWebServiceRequest::ck_default("StudentInfo".to_string(), String::new(), token),
        token,
    )
    .await?;

    let mut si: StudentInfo_ = quick_xml::de::from_str(result.as_str())?;
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
                .emergency_contact
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

// Xml structs

#[derive(Serialize, Deserialize)]
pub struct StudentInfo_ {
    #[serde(rename = "@xmlns:xsd")]
    pub xmlns_xsd: String,
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "@Type")]
    pub student_info_type: String,
    #[serde(rename = "@ShowStudentBusAssignmentInfo")]
    pub show_student_bus_assignment_info: String,
    #[serde(rename = "@ShowPhysicianAndDentistInfo")]
    pub show_physician_and_dentist_info: String,
    #[serde(rename = "@ShowStudentInfo")]
    pub show_student_info: String,
    #[serde(rename = "@ShowFrontLineSpedURL")]
    pub show_front_line_sped_url: String,
    #[serde(rename = "@ShowFrontLine504URL")]
    pub show_front_line504_url: String,
    #[serde(rename = "@ShowFrontLineParentPortalURL")]
    pub show_front_line_parent_portal_url: String,
    #[serde(rename = "LockerInfoRecords")]
    pub locker_info_records: LockerInfoRecords,
    #[serde(rename = "FormattedName")]
    pub formatted_name: String,
    #[serde(rename = "PermID")]
    pub perm_id: String,
    #[serde(rename = "Gender")]
    pub gender: String,
    #[serde(rename = "Grade")]
    pub grade: String,
    #[serde(rename = "Address")]
    pub address: String,
    #[serde(rename = "LastNameGoesBy")]
    pub last_name_goes_by: LastNameGoesBy,
    #[serde(rename = "NickName")]
    pub nick_name: String,
    #[serde(rename = "BirthDate")]
    pub birth_date: String,
    #[serde(rename = "EMail")]
    pub email: String,
    #[serde(rename = "Phone")]
    pub phone: String,
    #[serde(rename = "HomeLanguage")]
    pub home_language: HomeLanguage,
    #[serde(rename = "CurrentSchool")]
    pub current_school: String,
    #[serde(rename = "Track")]
    pub track: Track,
    #[serde(rename = "HomeRoomTch")]
    pub home_room_tch: String,
    #[serde(rename = "HomeRoomTchEMail")]
    pub home_room_tch_email: String,
    #[serde(rename = "HomeRoomTchStaffGU")]
    pub home_room_tch_staff_gu: String,
    #[serde(rename = "OrgYearGU")]
    pub org_year_gu: String,
    #[serde(rename = "HomeRoom")]
    pub home_room: String,
    #[serde(rename = "CounselorName")]
    pub counselor_name: String,
    #[serde(rename = "CounselorEmail")]
    pub counselor_email: String,
    #[serde(rename = "CounselorStaffGU")]
    pub counselor_staff_gu: String,
    #[serde(rename = "Photo")]
    #[serde(deserialize_with = "base64")]
    pub photo: Vec<u8>,
    #[serde(rename = "EmergencyContacts")]
    pub emergency_contacts: EmergencyContacts,
    #[serde(rename = "Physician")]
    pub physician: Physician,
    #[serde(rename = "Dentist")]
    pub dentist: Dentist,
    #[serde(rename = "UserDefinedGroupBoxes")]
    pub user_defined_group_boxes: UserDefinedGroupBoxes,
    #[serde(rename = "StudentBusAssignments")]
    pub student_bus_assignments: StudentBusAssignments,
}

#[derive(Serialize, Deserialize)]
pub struct LockerInfoRecords {
    #[serde(rename = "StudentLockerInfoRecord")]
    #[serde(default)]
    pub student_locker_info_record: Option<StudentLockerInfoRecord>,
}

#[derive(Serialize, Deserialize)]
pub struct StudentLockerInfoRecord {
    #[serde(rename = "@LockerGU")]
    pub locker_gu: String,
    #[serde(rename = "@LockerNumber")]
    pub locker_number: String,
    #[serde(rename = "@CurrentCombination")]
    pub current_combination: String,
    #[serde(rename = "@Location")]
    pub location: String,
}

#[derive(Serialize, Deserialize)]
pub struct LastNameGoesBy {}

#[derive(Serialize, Deserialize)]
pub struct HomeLanguage {}

#[derive(Serialize, Deserialize)]
pub struct Track {}

#[derive(Serialize, Deserialize)]
pub struct EmergencyContacts {
    #[serde(rename = "EmergencyContact")]
    pub emergency_contact: Vec<EmergencyContact>,
}

#[derive(Serialize, Deserialize)]
pub struct EmergencyContact {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Relationship")]
    pub relationship: String,
    #[serde(rename = "@HomePhone")]
    pub home_phone: String,
    #[serde(rename = "@WorkPhone")]
    pub work_phone: String,
    #[serde(rename = "@OtherPhone")]
    pub other_phone: String,
    #[serde(rename = "@MobilePhone")]
    pub mobile_phone: String,
}

#[derive(Serialize, Deserialize)]
pub struct Physician {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Hospital")]
    pub hospital: String,
    #[serde(rename = "@Phone")]
    pub phone: String,
    #[serde(rename = "@Extn")]
    pub extn: String,
}

#[derive(Serialize, Deserialize)]
pub struct Dentist {
    #[serde(rename = "@Name")]
    pub name: String,
    #[serde(rename = "@Office")]
    pub office: String,
    #[serde(rename = "@Phone")]
    pub phone: String,
    #[serde(rename = "@Extn")]
    pub extn: String,
}

#[derive(Serialize, Deserialize)]
pub struct UserDefinedGroupBoxes {
    #[serde(rename = "UserDefinedGroupBox")]
    pub user_defined_group_box: Vec<UserDefinedGroupBox>,
}

#[derive(Serialize, Deserialize)]
pub struct UserDefinedGroupBox {
    #[serde(rename = "@GroupBoxLabel")]
    pub group_box_label: String,
    #[serde(rename = "UserDefinedItems")]
    pub user_defined_items: Vec<UserDefinedItems>,
}

#[derive(Serialize, Deserialize)]
pub struct UserDefinedItems {
    #[serde(rename = "UserDefinedItem")]
    pub user_defined_item: Vec<UserDefinedItem>,
}

#[derive(Serialize, Deserialize)]
pub struct UserDefinedItem {
    #[serde(rename = "@ItemLabel")]
    pub item_label: String,
    #[serde(rename = "@ItemType")]
    pub item_type: String,
    #[serde(rename = "@SourceObject")]
    pub source_object: String,
    #[serde(rename = "@SourceElement")]
    pub source_element: String,
    #[serde(rename = "@VCID")]
    pub vcid: String,
    #[serde(rename = "@Value")]
    pub value: String,
}

#[derive(Serialize, Deserialize)]
pub struct StudentBusAssignments {}
