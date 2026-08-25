use std::path::{Path, PathBuf};

use reqwest::multipart::{Form, Part};

use crate::{
    Result,
    model::{AttachmentRequest, AttachmentRequestId},
};

/// Source used for a multipart Discord file upload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadSource {
    /// In-memory file contents.
    Bytes(Vec<u8>),
    /// A file opened and streamed from disk for each request attempt.
    Path(PathBuf),
}

/// Rebuildable descriptor for one Discord `files[n]` multipart upload.
///
/// Descriptors are intentionally cloneable so a request can rebuild its multipart
/// form after Discord returns a rate limit response without buffering file-backed
/// uploads into memory.
#[derive(Debug, Clone, PartialEq)]
pub struct UploadFile {
    pub id: u32,
    pub filename: String,
    pub source: UploadSource,
    pub title: Option<String>,
    pub description: Option<String>,
    pub content_type: Option<String>,
    pub duration_secs: Option<f64>,
    pub waveform: Option<String>,
    pub spoiler: bool,
}

impl UploadFile {
    /// Creates an in-memory upload descriptor.
    #[must_use]
    pub fn bytes(id: u32, filename: impl Into<String>, bytes: impl Into<Vec<u8>>) -> Self {
        Self {
            id,
            filename: filename.into(),
            source: UploadSource::Bytes(bytes.into()),
            title: None,
            description: None,
            content_type: None,
            duration_secs: None,
            waveform: None,
            spoiler: false,
        }
    }

    /// Creates a file-backed upload descriptor that is streamed from disk.
    #[must_use]
    pub fn path(id: u32, filename: impl Into<String>, path: impl AsRef<Path>) -> Self {
        Self {
            id,
            filename: filename.into(),
            source: UploadSource::Path(path.as_ref().to_owned()),
            title: None,
            description: None,
            content_type: None,
            duration_secs: None,
            waveform: None,
            spoiler: false,
        }
    }

    /// Sets the attachment title sent to Discord.
    #[must_use]
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets attachment alt text.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets an explicit MIME content type for this file part.
    #[must_use]
    pub fn content_type(mut self, content_type: impl Into<String>) -> Self {
        self.content_type = Some(content_type.into());
        self
    }

    /// Marks the uploaded attachment as a spoiler.
    #[must_use]
    pub const fn spoiler(mut self, spoiler: bool) -> Self {
        self.spoiler = spoiler;
        self
    }

    /// Sets voice/video duration metadata.
    #[must_use]
    pub const fn duration_secs(mut self, duration_secs: f64) -> Self {
        self.duration_secs = Some(duration_secs);
        self
    }

    /// Sets base64 waveform metadata used by voice messages.
    #[must_use]
    pub fn waveform(mut self, waveform: impl Into<String>) -> Self {
        self.waveform = Some(waveform.into());
        self
    }

    pub(crate) fn attachment_request(&self) -> AttachmentRequest {
        AttachmentRequest {
            id: AttachmentRequestId::Upload(self.id),
            filename: Some(self.filename.clone()),
            title: self.title.clone(),
            description: self.description.clone(),
            duration_secs: self.duration_secs,
            waveform: self.waveform.clone(),
            is_spoiler: Some(self.spoiler),
        }
    }

    async fn part(&self) -> Result<Part> {
        let mut part = match &self.source {
            UploadSource::Bytes(bytes) => Part::bytes(bytes.clone()),
            UploadSource::Path(path) => Part::file(path).await?,
        }
        .file_name(self.filename.clone());

        if let Some(content_type) = &self.content_type {
            part = part.mime_str(content_type)?;
        }

        Ok(part)
    }
}

pub(crate) async fn multipart_form(payload_json: String, files: &[UploadFile]) -> Result<Form> {
    let mut form = Form::new().text("payload_json", payload_json);

    for file in files {
        form = form.part(format!("files[{}]", file.id), file.part().await?);
    }

    Ok(form)
}

#[cfg(test)]
mod tests {
    use super::UploadFile;
    use crate::model::AttachmentRequestId;

    #[test]
    fn upload_metadata_uses_matching_file_index() {
        let upload = UploadFile::bytes(3, "image.png", [1, 2, 3])
            .description("alt")
            .spoiler(true);
        let attachment = upload.attachment_request();

        assert_eq!(attachment.id, AttachmentRequestId::Upload(3));
        assert_eq!(attachment.filename.as_deref(), Some("image.png"));
        assert_eq!(attachment.description.as_deref(), Some("alt"));
        assert_eq!(attachment.is_spoiler, Some(true));
    }
}
