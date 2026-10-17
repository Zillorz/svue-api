use crate::{
    api::{api_request, ApiError},
    crypto::AuthToken,
};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;

pub async fn list_documents(token: &mut AuthToken) -> Result<Vec<Document>, ApiError> {
    let data: StudentDocumentsData = api_request(
        "GetStudentDocuments",
        json!({"childIntID": 0, "languageCode": "en"}),
        token,
    )
    .await?;

    Ok(data
        .student_documents
        .student_document_datas
        .into_iter()
        .map(|x| x.into())
        .collect())
}

pub async fn get_document(token: &mut AuthToken, gu: String) -> Result<DocumentData, ApiError> {
    let data: StudentAttachedDocumentDataData = api_request(
        "GetStudentDocumentContent",
        json!({"childIntID": 0, "documentGU": gu}),
        token,
    )
    .await?;

    Ok(data
        .student_attached_document_data
        .document_datas
        .into_iter()
        .next()
        .ok_or(ApiError::NotFound)?
        .into())
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudentDocumentsData {
    student_documents: StudentDocuments,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudentDocuments {
    #[serde(default)]
    student_document_datas: Vec<StudentDocumentData>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StudentDocumentData {
    #[serde(rename = "documentGU")]
    pub document_gu: String,
    pub document_file_name: String,
    pub document_date: String,
    pub document_comment: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudentAttachedDocumentDataData {
    student_attached_document_data: StudentAttachedDocumentData,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StudentAttachedDocumentData {
    #[serde(default)]
    document_datas: Vec<DocumentData_>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentData_ {
    pub file_name: String,
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
