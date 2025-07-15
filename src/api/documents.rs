use crate::{
    api::{api_request, ApiError, ProcessWebServiceRequest},
    crypto::AuthToken,
};
use base64::prelude::BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Deserializer, Serialize};

pub async fn list_documents(token: &mut AuthToken) -> Result<Vec<Document>, ApiError> {
    let result = api_request(
        ProcessWebServiceRequest::ck_default(
            "GetStudentDocumentInitialData".to_string(),
            String::new(),
            token,
        ),
        token,
    )
    .await?;

    let docs: StudentDocuments = quick_xml::de::from_str(result.as_str())?;
    Ok(docs
        .student_document_datas
        .student_document_data
        .into_iter()
        .map(|x| x.into())
        .collect())
}

pub async fn get_document(token: &mut AuthToken, gu: String) -> Result<DocumentData, ApiError> {
    let result = api_request(
        ProcessWebServiceRequest::ck_default(
            "GetContentOfAttachedDoc".to_string(),
            format!("<DocumentGU>{gu}</DocumentGU>"),
            token,
        ),
        token,
    )
    .await?;

    let docs: StudentAttachedDocumentData = quick_xml::de::from_str(result.as_str())?;
    Ok(docs.document_datas.document_data.into())
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
pub struct StudentDocuments {
    #[serde(rename = "@xmlns:xsd")]
    pub xmlns_xsd: String,
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "@showDateColumn")]
    pub show_date_column: String,
    #[serde(rename = "@showDocNameColumn")]
    pub show_doc_name_column: String,
    #[serde(rename = "@showDocCatColumn")]
    pub show_doc_cat_column: String,
    #[serde(rename = "@StudentGU")]
    pub student_gu: String,
    #[serde(rename = "@StudentSSY")]
    pub student_ssy: String,
    #[serde(rename = "StudentDocumentDatas")]
    pub student_document_datas: StudentDocumentDatas,
}

#[derive(Serialize, Deserialize)]
pub struct StudentDocumentDatas {
    #[serde(rename = "StudentDocumentData")]
    pub student_document_data: Vec<StudentDocumentData>,
}

#[derive(Serialize, Deserialize)]
pub struct StudentDocumentData {
    #[serde(rename = "@DocumentGU")]
    pub document_gu: String,
    #[serde(rename = "@DocumentFileName")]
    pub document_file_name: String,
    #[serde(rename = "@DocumentDate")]
    pub document_date: String,
    #[serde(rename = "@DocumentType")]
    pub document_type: String,
    #[serde(rename = "@StudentGU")]
    pub student_gu: String,
    #[serde(rename = "@DocumentComment")]
    pub document_comment: String,
}

#[derive(Serialize, Deserialize)]
pub struct StudentAttachedDocumentData {
    #[serde(rename = "@xmlns:xsd")]
    pub xmlns_xsd: String,
    #[serde(rename = "@xmlns:xsi")]
    pub xmlns_xsi: String,
    #[serde(rename = "DocumentDatas")]
    pub document_datas: DocumentDatas,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentDatas {
    #[serde(rename = "DocumentData")]
    pub document_data: DocumentData_,
}

#[derive(Serialize, Deserialize)]
pub struct DocumentData_ {
    #[serde(rename = "@DocumentGU")]
    pub document_gu: String,
    #[serde(rename = "@StudentGU")]
    pub student_gu: String,
    #[serde(rename = "@DocDate")]
    pub doc_date: String,
    #[serde(rename = "@FileName")]
    pub file_name: String,
    #[serde(rename = "@Category")]
    pub category: String,
    #[serde(rename = "@Notes")]
    pub notes: String,
    #[serde(rename = "@DocType")]
    pub doc_type: String,
    #[serde(rename = "Base64Code")]
    #[serde(deserialize_with = "base64")]
    pub data: Vec<u8>,
}

pub fn base64<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
    let base64 = String::deserialize(d)?;
    BASE64_STANDARD
        .decode(base64.as_bytes())
        .map_err(serde::de::Error::custom)
}
