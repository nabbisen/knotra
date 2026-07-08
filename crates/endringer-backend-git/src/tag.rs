use anyhow::{Context, Result};
use gix::bstr::ByteSlice;
use endringer_backend_core::types::{CommitId, SortOrder, TagAnnotation, TagInfo};
use gix::Repository;

use crate::util::{gix_id_to_commit_id, seconds_to_systemtime};

const TAGS_PREFIX: &str = "refs/tags/";

pub(crate) fn list_tags(repository: &Repository) -> Result<Vec<TagInfo>> {
    let mut tags = Vec::new();

    for reference in repository.references()?.prefixed(TAGS_PREFIX)? {
        let mut reference = reference.map_err(|e| anyhow::anyhow!("{e}"))?;
        let full_name = reference.name().as_bstr().to_string();
        let name = full_name
            .strip_prefix(TAGS_PREFIX)
            .unwrap_or(&full_name)
            .to_owned();

        // Before peeling, check if the ref points to a tag object (annotated)
        // or directly to a commit (lightweight).
        let annotation = read_annotation(repository, &reference);

        let commit = reference.peel_to_commit()?;
        let commit_id: CommitId = gix_id_to_commit_id(commit.id);
        let commit_summary = commit
            .message()
            .context("failed to read commit message")?
            .summary()
            .to_string();
        let commit_timestamp =
            seconds_to_systemtime(commit.time().context("failed to read commit timestamp")?.seconds);

        tags.push(TagInfo { name, full_name, commit_id, commit_summary, commit_timestamp, annotation });
    }

    Ok(tags)
}

/// Reads annotation metadata from the tag object the reference currently points
/// to. Returns `None` for lightweight tags (which point directly to a commit).
fn read_annotation(repository: &Repository, reference: &gix::Reference<'_>) -> Option<TagAnnotation> {
    let raw_oid = reference.try_id()?.detach();
    let obj = repository.find_object(raw_oid).ok()?;
    if obj.kind != gix::object::Kind::Tag {
        return None; // lightweight tag
    }
    let tag = obj.try_into_tag().ok()?;
    let message = tag.decode().ok().map(|d| std::str::from_utf8(d.message).map(str::trim).unwrap_or("").to_string())?;
    let (tagger_name, tagger_timestamp) = if let Ok(Some(sig)) = tag.tagger() {
        let name = sig.name.to_str_lossy().into_owned();
        let ts = sig.time().ok().map(|t| seconds_to_systemtime(t.seconds));
        (Some(name), ts)
    } else {
        (None, None)
    };
    Some(TagAnnotation { message, tagger_name, tagger_timestamp })
}

pub(crate) fn list_tags_sorted(repository: &Repository, order: SortOrder) -> Result<Vec<TagInfo>> {
    let mut tags = list_tags(repository)?;
    match order {
        SortOrder::NewestFirst => tags.sort_by(|a, b| b.commit_timestamp.cmp(&a.commit_timestamp)),
        SortOrder::OldestFirst => tags.sort_by(|a, b| a.commit_timestamp.cmp(&b.commit_timestamp)),
        SortOrder::ByName => tags.sort_by(|a, b| a.name.cmp(&b.name)),
    }
    Ok(tags)
}

pub(crate) fn create_tag(repository: &Repository, name: &str) -> Result<()> {
    let head_id = repository
        .head()?
        .id()
        .ok_or_else(|| anyhow::anyhow!("HEAD is not pointing to a commit"))?;
    repository
        .reference(
            format!("{}{}", TAGS_PREFIX, name).as_str(),
            head_id.detach(),
            gix::refs::transaction::PreviousValue::MustNotExist,
            format!("tag: created lightweight tag {}", name),
        )
        .with_context(|| format!("failed to create tag '{}'", name))?;
    Ok(())
}

pub(crate) fn create_annotated_tag(repository: &Repository, name: &str, message: &str) -> Result<()> {
    let head_id = repository
        .head()?
        .id()
        .ok_or_else(|| anyhow::anyhow!("HEAD is not pointing to a commit"))?
        .detach();

    let tagger = repository
        .committer()
        .ok_or_else(|| {
            anyhow::anyhow!(
                "no committer identity — set user.name and user.email in git config"
            )
        })?
        .context("failed to resolve committer identity")?;

    repository
        .tag(
            name,
            &head_id,
            gix::object::Kind::Commit,
            Some(tagger),
            message,
            gix::refs::transaction::PreviousValue::MustNotExist,
        )
        .with_context(|| format!("failed to create annotated tag '{}'", name))?;
    Ok(())
}

pub(crate) fn delete_tag(repository: &Repository, name: &str) -> Result<()> {
    let full_ref = format!("{}{}", TAGS_PREFIX, name);
    let edit = gix::refs::transaction::RefEdit {
        change: gix::refs::transaction::Change::Delete {
            expected: gix::refs::transaction::PreviousValue::Any,
            log: gix::refs::transaction::RefLog::AndReference,
        },
        name: full_ref
            .as_str()
            .try_into()
            .map_err(|_| anyhow::anyhow!("invalid ref name '{}'", full_ref))?,
        deref: false,
    };
    repository
        .edit_references(std::iter::once(edit))
        .with_context(|| format!("failed to delete tag '{}'", name))?;
    Ok(())
}
