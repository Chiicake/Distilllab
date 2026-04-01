use schema::AttachmentRef;
use std::fs;
use std::path::{Path, PathBuf};
use uuid::Uuid;

type FlowError = Box<dyn std::error::Error + Send + Sync>;

pub fn store_attachment_copy(
    storage_root: &Path,
    session_id: &str,
    original_path: &str,
) -> Result<AttachmentRef, FlowError> {
    let original = PathBuf::from(original_path);
    let file_name = original
        .file_name()
        .and_then(|value| value.to_str())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "invalid attachment file name",
            )
        })?
        .to_string();

    let attachment_id = format!("attachment-{}", Uuid::new_v4());
    let target_dir = storage_root
        .join("attachments")
        .join(session_id)
        .join(&attachment_id);
    fs::create_dir_all(&target_dir)?;
    let target_path = target_dir.join(&file_name);
    fs::copy(&original, &target_path)?;
    let metadata = fs::metadata(&target_path)?;

    Ok(AttachmentRef {
        attachment_id,
        kind: "file_path".to_string(),
        name: file_name,
        mime_type: "application/octet-stream".to_string(),
        path_or_locator: target_path.to_string_lossy().to_string(),
        size: metadata.len(),
        metadata_json: format!(
            "{{\"original_path\":\"{}\"}}",
            original_path.replace('"', "\\\"")
        ),
    })
}

pub fn remove_session_attachment_storage(
    storage_root: &Path,
    session_id: &str,
) -> Result<(), FlowError> {
    let session_attachment_dir = storage_root.join("attachments").join(session_id);

    match fs::remove_dir_all(&session_attachment_dir) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(Box::new(error)),
    }
}

#[cfg(test)]
mod tests {
    use super::{remove_session_attachment_storage, store_attachment_copy};
    use std::fs;
    use uuid::Uuid;

    #[test]
    fn stores_attachment_copy_under_distilllab_managed_data_directory() {
        let temp_root =
            std::env::temp_dir().join(format!("distilllab-attachment-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).expect("temp root should be created");

        let original_path = temp_root.join("notes.md");
        fs::write(&original_path, "runtime design notes").expect("source file should be written");

        let attachment = store_attachment_copy(
            &temp_root,
            "session-1",
            original_path.to_string_lossy().as_ref(),
        )
        .expect("attachment should be stored");

        assert_eq!(attachment.name, "notes.md");
        assert_ne!(attachment.path_or_locator, original_path.to_string_lossy());
        assert!(attachment.path_or_locator.contains("attachments"));
        assert!(std::path::Path::new(&attachment.path_or_locator).exists());
    }

    #[test]
    fn removes_session_attachment_storage_directory_when_present() {
        let temp_root =
            std::env::temp_dir().join(format!("distilllab-attachment-cleanup-{}", Uuid::new_v4()));
        fs::create_dir_all(&temp_root).expect("temp root should be created");

        let original_path = temp_root.join("notes.md");
        fs::write(&original_path, "runtime design notes").expect("source file should be written");

        let attachment = store_attachment_copy(
            &temp_root,
            "session-cleanup",
            original_path.to_string_lossy().as_ref(),
        )
        .expect("attachment should be stored");

        let session_dir = temp_root.join("attachments").join("session-cleanup");
        assert!(session_dir.exists());
        assert!(std::path::Path::new(&attachment.path_or_locator).exists());

        remove_session_attachment_storage(&temp_root, "session-cleanup")
            .expect("attachment storage should be removed");

        assert!(!session_dir.exists());
    }

    #[test]
    fn removing_missing_session_attachment_storage_is_a_noop() {
        let temp_root = std::env::temp_dir().join(format!(
            "distilllab-attachment-missing-cleanup-{}",
            Uuid::new_v4()
        ));
        fs::create_dir_all(&temp_root).expect("temp root should be created");

        remove_session_attachment_storage(&temp_root, "session-missing")
            .expect("missing attachment storage should not error");
    }
}
