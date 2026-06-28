use std::path::{Path, PathBuf};
use std::process::Command;

use crate::models::StaticSitePost;
use crate::security::redact::redact_secrets;

#[derive(Debug, Clone)]
pub struct FireflyPublishRequest {
    pub repo_path: PathBuf,
    pub posts_dir: String,
    pub build_command: String,
    pub commit_message: String,
    pub remote_name: String,
    pub branch: Option<String>,
    pub push_enabled: bool,
    pub validate_build: bool,
    pub dry_run: bool,
    pub post: StaticSitePost,
}

#[derive(Debug, Clone)]
pub struct FireflyPublishResult {
    pub post_path: String,
    pub commit_id: Option<String>,
    pub command_log: Vec<String>,
}

pub fn sanitize_post_slug(input: &str) -> String {
    let mut parts = Vec::new();
    let mut current = String::new();
    for ch in input.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            current.push(ch.to_ascii_lowercase());
        } else if ch == '-' || ch == '_' || ch == '/' {
            push_slug_part(&mut parts, &mut current);
            if ch == '/' {
                parts.push("/".to_string());
            }
        } else if ch.is_whitespace() {
            push_slug_part(&mut parts, &mut current);
        } else if let Some(word) = chinese_slug_word(ch) {
            push_slug_part(&mut parts, &mut current);
            parts.push(word.to_string());
        } else {
            push_slug_part(&mut parts, &mut current);
        }
    }
    push_slug_part(&mut parts, &mut current);
    let slug = parts
        .into_iter()
        .fold(String::new(), |mut acc, part| {
            if part == "/" {
                while acc.ends_with('-') {
                    acc.pop();
                }
                if !acc.ends_with('/') && !acc.is_empty() {
                    acc.push('/');
                }
                return acc;
            }
            if !acc.is_empty() && !acc.ends_with('/') {
                acc.push('-');
            }
            acc.push_str(&part);
            acc
        })
        .trim_matches(['-', '/'])
        .to_string();
    if slug.is_empty() {
        "untitled".to_string()
    } else {
        slug
    }
}

fn push_slug_part(parts: &mut Vec<String>, current: &mut String) {
    if !current.is_empty() {
        parts.push(std::mem::take(current));
    }
}

fn chinese_slug_word(ch: char) -> Option<&'static str> {
    match ch {
        '雨' => Some("yu"),
        '夜' => Some("ye"),
        '旧' => Some("jiu"),
        '案' => Some("an"),
        _ => None,
    }
}

pub fn render_firefly_markdown(post: &StaticSitePost) -> String {
    let slug = sanitize_post_slug(&post.slug);
    let body = redact_publish_body(&post.body_markdown);
    let category = post.category.as_deref().unwrap_or("");
    let lang = post.lang.as_deref().unwrap_or("zh-CN");
    let tags = post
        .tags
        .iter()
        .map(|tag| yaml_string(tag))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "---\n\
title: {}\n\
published: {}\n\
description: {}\n\
image: \"\"\n\
tags: [{}]\n\
category: {}\n\
draft: false\n\
lang: {}\n\
comment: true\n\
---\n\n\
<!-- generated-slug: {} -->\n\n\
{}\n",
        yaml_string(&post.title),
        yaml_string(&post.published),
        yaml_string(&post.description),
        tags,
        yaml_string(category),
        yaml_string(lang),
        slug,
        body
    )
}

fn yaml_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\r', " ")
        .replace('\n', " ");
    format!("\"{}\"", escaped.trim())
}

fn redact_publish_body(body: &str) -> String {
    body.lines()
        .filter(|line| {
            let lower = line.to_ascii_lowercase();
            !(lower.contains("api_key")
                || lower.contains("secret")
                || lower.contains("token")
                || lower.contains("password"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn post_path(posts_dir: &str, slug: &str) -> PathBuf {
    Path::new(posts_dir).join(format!("{}.md", sanitize_post_slug(slug)))
}

pub fn validate_firefly_target(repo: &Path, posts_dir: &str) -> Result<(), String> {
    if !repo.join(".git").exists() {
        return Err(format!(
            "Publication target is not a Git repository: {}",
            repo.display()
        ));
    }
    if !repo.join("src").join("content.config.ts").exists() {
        return Err("Firefly target is missing src/content.config.ts".to_string());
    }
    let posts = repo.join(posts_dir);
    if let Err(e) = std::fs::create_dir_all(&posts) {
        return Err(format!("Create Firefly posts directory: {}", e));
    }
    Ok(())
}

pub fn write_firefly_post(
    repo: &Path,
    posts_dir: &str,
    post: &StaticSitePost,
) -> Result<PathBuf, String> {
    validate_firefly_target(repo, posts_dir)?;
    let relative = post_path(posts_dir, &post.slug);
    let full_path = repo.join(&relative);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("Create post parent dir: {}", e))?;
    }
    std::fs::write(&full_path, render_firefly_markdown(post))
        .map_err(|e| format!("Write Firefly post: {}", e))?;
    Ok(relative)
}

pub fn plan_firefly_git_steps(
    _repo: &Path,
    generated_post_path: &str,
    build_command: &str,
    commit_message: &str,
    remote_name: &str,
    branch: Option<&str>,
    push_enabled: bool,
) -> Vec<Vec<String>> {
    let mut steps = Vec::new();
    if !build_command.trim().is_empty() {
        steps.push(split_command(build_command));
    }
    steps.push(vec![
        "git".to_string(),
        "status".to_string(),
        "--porcelain".to_string(),
    ]);
    steps.push(vec![
        "git".to_string(),
        "add".to_string(),
        "--".to_string(),
        generated_post_path.to_string(),
    ]);
    steps.push(vec![
        "git".to_string(),
        "commit".to_string(),
        "-m".to_string(),
        commit_message.to_string(),
    ]);
    if push_enabled {
        let mut push = vec![
            "git".to_string(),
            "push".to_string(),
            remote_name.to_string(),
        ];
        if let Some(branch) = branch.filter(|value| !value.trim().is_empty()) {
            push.push(branch.to_string());
        }
        steps.push(push);
    }
    steps
}

fn split_command(command: &str) -> Vec<String> {
    command
        .split_whitespace()
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect()
}

pub fn publish_firefly_git(
    request: &FireflyPublishRequest,
) -> Result<FireflyPublishResult, String> {
    validate_firefly_target(&request.repo_path, &request.posts_dir)?;
    let relative = if request.dry_run {
        post_path(&request.posts_dir, &request.post.slug)
    } else {
        write_firefly_post(&request.repo_path, &request.posts_dir, &request.post)?
    };
    let relative_string = normalize_relative_path(&relative);
    let mut command_log = Vec::new();

    if request.dry_run {
        return Ok(FireflyPublishResult {
            post_path: relative_string,
            commit_id: None,
            command_log,
        });
    }

    if request.validate_build && !request.build_command.trim().is_empty() {
        let build = split_command(&request.build_command);
        run_command(&request.repo_path, &build, &mut command_log)?;
    }

    let status = run_command(
        &request.repo_path,
        &[
            "git".to_string(),
            "status".to_string(),
            "--porcelain".to_string(),
        ],
        &mut command_log,
    )?;
    ensure_only_generated_file_changed(&status, &relative_string)?;
    run_command(
        &request.repo_path,
        &[
            "git".to_string(),
            "add".to_string(),
            "--".to_string(),
            relative_string.clone(),
        ],
        &mut command_log,
    )?;
    run_command(
        &request.repo_path,
        &[
            "git".to_string(),
            "commit".to_string(),
            "-m".to_string(),
            request.commit_message.clone(),
        ],
        &mut command_log,
    )?;
    let commit_id = run_command(
        &request.repo_path,
        &[
            "git".to_string(),
            "rev-parse".to_string(),
            "HEAD".to_string(),
        ],
        &mut command_log,
    )?
    .trim()
    .to_string();
    if request.push_enabled {
        let mut push = vec![
            "git".to_string(),
            "push".to_string(),
            request.remote_name.clone(),
        ];
        if let Some(branch) = request
            .branch
            .as_deref()
            .filter(|value| !value.trim().is_empty())
        {
            push.push(branch.to_string());
        }
        run_command(&request.repo_path, &push, &mut command_log)?;
    }

    Ok(FireflyPublishResult {
        post_path: relative_string,
        commit_id: Some(commit_id),
        command_log,
    })
}

fn normalize_relative_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn ensure_only_generated_file_changed(status: &str, generated_path: &str) -> Result<(), String> {
    let unrelated = status
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter(|line| {
            let path = line.get(3..).unwrap_or("").replace('\\', "/");
            path != generated_path
        })
        .collect::<Vec<_>>();
    if unrelated.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "Publication target has unrelated changes; refusing to publish: {}",
            unrelated.join(", ")
        ))
    }
}

fn run_command(
    cwd: &Path,
    args: &[String],
    command_log: &mut Vec<String>,
) -> Result<String, String> {
    let Some((program, rest)) = args.split_first() else {
        return Err("Empty command".to_string());
    };
    command_log.push(redact_secrets(&format!("$ {}", args.join(" "))));
    let output = Command::new(program)
        .args(rest)
        .current_dir(cwd)
        .output()
        .map_err(|e| format!("Run command {}: {}", program, e))?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !stdout.trim().is_empty() {
        command_log.push(redact_secrets(stdout.trim()));
    }
    if !stderr.trim().is_empty() {
        command_log.push(redact_secrets(stderr.trim()));
    }
    if output.status.success() {
        Ok(stdout)
    } else {
        Err(format!(
            "Command failed ({}): {}",
            args.join(" "),
            redact_secrets(stderr.trim())
        ))
    }
}
