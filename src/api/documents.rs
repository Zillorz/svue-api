use crate::{
    api::{ApiError, DefaultReq, api_req},
    crypto::AuthToken,
};
use base64::Engine;
use base64::prelude::BASE64_STANDARD;
use serde::{Deserialize, Deserializer, Serialize};

pub async fn list_documents(token: &mut AuthToken) -> Result<Vec<Document>, ApiError> {
    let result: OuterDocs = api_req("GetStudentDocuments", DefaultReq::default(), token).await?;

    Ok(result
        .inner
        .student_document_datas
        .into_iter()
        .map(|x| x.into())
        .collect())
}

#[derive(Serialize, Deserialize)]
struct DocumentReq {
    #[serde(rename = "childIntID")]
    child_int_id: i32,
    #[serde(rename = "documentGU")]
    document_gu: String,
}

pub async fn get_document(token: &mut AuthToken, gu: String) -> Result<DocumentData, ApiError> {
    let result: OuterDoc = api_req(
        "GetStudentDocumentContent",
        DocumentReq {
            child_int_id: 0,
            document_gu: gu,
        },
        token,
    )
    .await?;

    let doc = result
        .inner
        .document_datas
        .into_iter()
        .next()
        .ok_or(ApiError::Unknown)?;

    Ok(doc.into())
}

// Api structs
#[derive(Serialize, Deserialize, Debug)]
pub struct Document {
    pub name: String,
    file_name: String,
    date: String,
    pub gu: String,
}

impl From<StudentDocumentData> for Document {
    fn from(value: StudentDocumentData) -> Self {
        Document {
            name: value.document_comment,
            file_name: value.document_file_name,
            date: value.document_date,
            gu: value.document_gu,
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct DocumentData {
    pub file_name: String,
    pub file_data: Vec<u8>,
}

impl From<DocumentData_> for DocumentData {
    fn from(value: DocumentData_) -> Self {
        DocumentData {
            file_name: value.file_name,
            file_data: value.data,
        }
    }
}

// XML structs

#[derive(Serialize, Deserialize)]
pub struct OuterDocs {
    #[serde(rename = "studentDocuments")]
    inner: StudentDocuments,
}

#[derive(Serialize, Deserialize)]
pub struct StudentDocuments {
    #[serde(rename = "showDateColumn")]
    pub show_date_column: bool,
    #[serde(rename = "showDocNameColumn")]
    pub show_doc_name_column: bool,
    #[serde(rename = "showDocCatColumn")]
    pub show_doc_cat_column: bool,
    #[serde(rename = "studentGU")]
    pub student_gu: String,
    #[serde(rename = "studentSSY")]
    pub student_ssy: String,
    #[serde(rename = "studentDocumentDatas")]
    pub student_document_datas: Vec<StudentDocumentData>,
}

#[derive(Serialize, Deserialize)]
pub struct StudentDocumentData {
    #[serde(rename = "documentGU")]
    pub document_gu: String,
    #[serde(rename = "documentFileName")]
    pub document_file_name: String,
    #[serde(rename = "documentDate")]
    pub document_date: String,
    #[serde(rename = "documentType")]
    pub document_type: String,
    #[serde(rename = "studentGU")]
    pub student_gu: String,
    #[serde(rename = "documentComment")]
    pub document_comment: String,
}

#[derive(Serialize, Deserialize)]
pub struct OuterDoc {
    #[serde(rename = "studentAttachedDocumentData")]
    inner: StudentAttachedDocumentData,
}

#[derive(Serialize, Deserialize)]
pub struct StudentAttachedDocumentData {
    #[serde(rename = "documentDatas")]
    pub document_datas: Vec<DocumentData_>,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentData_ {
    #[serde(rename = "documentGU")]
    pub document_gu: String,
    #[serde(rename = "studentGU")]
    pub student_gu: String,
    #[serde(rename = "docDate")]
    pub doc_date: String,
    #[serde(rename = "fileName")]
    pub file_name: String,
    #[serde(rename = "category")]
    pub category: String,
    #[serde(rename = "notes")]
    pub notes: String,
    #[serde(rename = "docType")]
    pub doc_type: String,
    #[serde(rename = "base64Code")]
    #[serde(deserialize_with = "base64")]
    pub data: Vec<u8>,
}

pub fn base64<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let base64 = String::deserialize(d)?;
    BASE64_STANDARD
        .decode(base64.as_bytes())
        .map_err(serde::de::Error::custom)
}
