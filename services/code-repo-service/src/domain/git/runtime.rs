use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, Result, bail};
use chrono::{DateTime, Utc};

use crate::models::{
    branch::BranchDefinition,
    commit::{CiRun, CommitDefinition, CreateCommitRequest},
    file::RepositoryFile,
    repository::{PackageKind, RepositoryDefinition},
};

#[derive(Debug, Clone)]
pub struct GitBranchMetadata {
    pub id: uuid::Uuid,
    pub base_branch: Option<String>,
    pub protected: bool,
}

pub fn ensure_storage_root(root: &Path) -> Result<()> {
    fs::create_dir_all(root).with_context(|| {
        format!(
            "failed to create repository storage root {}",
            root.display()
        )
    })
}

pub fn repository_path(root: &Path, repository_id: uuid::Uuid) -> PathBuf {
    root.join(repository_id.to_string())
}

pub fn initialize_repository(
    root: &Path,
    repository: &RepositoryDefinition,
) -> Result<(String, Vec<RepositoryFile>)> {
    ensure_storage_root(root)?;
    let repo_dir = repository_path(root, repository.id);
    if repo_dir.exists() {
        bail!(
            "repository storage already exists at {}",
            repo_dir.display()
        );
    }

    fs::create_dir_all(&repo_dir).with_context(|| {
        format!(
            "failed to create repository directory {}",
            repo_dir.display()
        )
    })?;
    run_command(
        Some(&repo_dir),
        "git",
        &["init", "-b", &repository.default_branch],
    )?;

    let author_email = author_email(&repository.owner);

    for (path, content) in scaffold_files(repository) {
        let file_path = repo_dir.join(&path);
        if let Some(parent) = file_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!(
                    "failed to create scaffold parent directory {}",
                    parent.display()
                )
            })?;
        }
        fs::write(&file_path, content)
            .with_context(|| format!("failed to write scaffold file {}", file_path.display()))?;
    }

    git(&repo_dir, &["add", "."])?;
    git_with_env(
        &repo_dir,
        &[
            ("GIT_AUTHOR_NAME", repository.owner.as_str()),
            ("GIT_AUTHOR_EMAIL", author_email.as_str()),
            ("GIT_COMMITTER_NAME", repository.owner.as_str()),
            ("GIT_COMMITTER_EMAIL", author_email.as_str()),
        ],
        &[
            "commit",
            "-m",
            "Initialize repository",
            "-m",
            "Bootstrap a real Git-backed repository for OpenFoundry package delivery.",
        ],
    )?;

    let head_sha = git(&repo_dir, &["rev-parse", "HEAD"])?;
    let files = list_files(root, repository.id, &repository.default_branch)?;
    Ok((head_sha.trim().to_string(), files))
}

pub fn list_branches(
    root: &Path,
    repository: &RepositoryDefinition,
    metadata: &BTreeMap<String, GitBranchMetadata>,
    pending_reviews: &BTreeMap<String, usize>,
) -> Result<Vec<BranchDefinition>> {
    let repo_dir = repo_dir(root, repository.id)?;
    let raw = git(
        &repo_dir,
        &[
            "for-each-ref",
            "--format=%(refname:short)\t%(objectname)\t%(committerdate:iso-strict)",
            "refs/heads",
        ],
    )?;

    let mut branches = raw
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut parts = line.split('\t');
            let name = parts.next().unwrap_or_default().to_string();
            let head_sha = parts.next().unwrap_or_default().to_string();
            let updated_at = parse_git_timestamp(parts.next().unwrap_or_default())?;
            let is_default = name == repository.default_branch;
            let base_branch = if is_default {
                None
            } else {
                metadata
                    .get(&name)
                    .and_then(|entry| entry.base_branch.clone())
                    .or_else(|| Some(repository.default_branch.clone()))
            };
            let protected = metadata
                .get(&name)
                .map(|entry| entry.protected)
                .unwrap_or(is_default);
            let ahead_by = if let Some(base_branch) = base_branch.as_deref() {
                count_ahead_by(&repo_dir, base_branch, &name)?
            } else {
                0
            };
            Ok(BranchDefinition {
                id: metadata
                    .get(&name)
                    .map(|entry| entry.id)
                    .unwrap_or_else(uuid::Uuid::now_v7),
                repository_id: repository.id,
                name: name.clone(),
                head_sha,
                base_branch,
                is_default,
                protected,
                ahead_by,
                pending_reviews: pending_reviews.get(&name).copied().unwrap_or_default(),
                updated_at,
            })
        })
        .collect::<Result<Vec<_>>>()?;

    branches.sort_by(|left, right| {
        right
            .is_default
            .cmp(&left.is_default)
            .then_with(|| right.updated_at.cmp(&left.updated_at))
            .then_with(|| left.name.cmp(&right.name))
    });

    Ok(branches)
}

pub fn create_branch(
    root: &Path,
    repository_id: uuid::Uuid,
    name: &str,
    base_branch: &str,
) -> Result<()> {
    let repo_dir = repo_dir(root, repository_id)?;
    git(&repo_dir, &["branch", name, base_branch])?;
    Ok(())
}

pub fn list_commits(
    root: &Path,
    repository: &RepositoryDefinition,
) -> Result<Vec<CommitDefinition>> {
    let repo_dir = repo_dir(root, repository.id)?;
    let branch_names = git(
        &repo_dir,
        &["for-each-ref", "--format=%(refname:short)", "refs/heads"],
    )?;

    let mut commits_by_sha = BTreeMap::new();
    for branch_name in branch_names.lines().filter(|line| !line.trim().is_empty()) {
        let raw = git(
            &repo_dir,
            &[
                "log",
                branch_name,
                "--date=iso-strict",
                "--numstat",
                "--format=%x1e%H%x1f%P%x1f%an%x1f%ae%x1f%aI%x1f%s",
            ],
        )?;

        for entry in raw.split('\u{1e}').filter(|entry| !entry.trim().is_empty()) {
            let mut lines = entry.lines();
            let header = lines.next().unwrap_or_default().trim();
            if header.is_empty() {
                continue;
            }

            let mut fields = header.split('\u{1f}');
            let sha = fields.next().unwrap_or_default().trim().to_string();
            if sha.is_empty() || commits_by_sha.contains_key(&sha) {
                continue;
            }
            let parents = fields.next().unwrap_or_default().trim().to_string();
            let author_name = fields.next().unwrap_or_default().trim().to_string();
            let author_email = fields.next().unwrap_or_default().trim().to_string();
            let created_at = parse_git_timestamp(fields.next().unwrap_or_default().trim())?;
            let title = fields.next().unwrap_or_default().trim().to_string();

            let mut additions = 0;
            let mut deletions = 0;
            let mut seen_paths = BTreeSet::new();
            for line in lines {
                let mut parts = line.split('\t');
                let add = parts.next().unwrap_or_default();
                let del = parts.next().unwrap_or_default();
                let path = parts.next().unwrap_or_default();
                if path.is_empty() {
                    continue;
                }
                seen_paths.insert(path.to_string());
                if let Ok(value) = add.parse::<i32>() {
                    additions += value;
                }
                if let Ok(value) = del.parse::<i32>() {
                    deletions += value;
                }
            }
            let files_changed = seen_paths.len() as i32;

            commits_by_sha.insert(
                sha.clone(),
                CommitDefinition {
                    id: uuid::Uuid::now_v7(),
                    repository_id: repository.id,
                    branch_name: branch_name.to_string(),
                    sha,
                    parent_sha: parents
                        .split_whitespace()
                        .next()
                        .map(|value| value.to_string())
                        .filter(|value| !value.is_empty()),
                    title,
                    description: String::new(),
                    author_name,
                    author_email,
                    files_changed,
                    additions,
                    deletions,
                    created_at,
                },
            );
        }
    }

    let mut commits = commits_by_sha.into_values().collect::<Vec<_>>();
    commits.sort_by(|left, right| {
        right
            .created_at
            .cmp(&left.created_at)
            .then_with(|| left.sha.cmp(&right.sha))
    });
    Ok(commits)
}

pub fn apply_commit(
    root: &Path,
    repository: &RepositoryDefinition,
    request: &CreateCommitRequest,
) -> Result<CommitDefinition> {
    let repo_dir = repo_dir(root, repository.id)?;
    git(&repo_dir, &["checkout", &request.branch_name])?;

    if request.files.is_empty() {
        let change_path = format!(
            ".openfoundry/changes/{}-{}.md",
            Utc::now().timestamp(),
            slug_fragment(&request.title)
        );
        let content = format!(
            "# {}\n\n{}\n\nAuthor: {}\nGenerated at: {}\n",
            request.title,
            if request.description.trim().is_empty() {
                "No description provided."
            } else {
                request.description.trim()
            },
            request.author_name,
            Utc::now().to_rfc3339(),
        );
        write_repo_file(&repo_dir, &change_path, &content)?;
    } else {
        for change in &request.files {
            if change.path.trim().is_empty() {
                bail!("commit file path cannot be empty");
            }
            let path = repo_dir.join(&change.path);
            if change.delete {
                if path.exists() {
                    fs::remove_file(&path)
                        .with_context(|| format!("failed to remove {}", path.display()))?;
                }
                continue;
            }
            write_repo_file(&repo_dir, &change.path, &change.content)?;
        }
    }

    git(&repo_dir, &["add", "-A"])?;
    let mut args = vec!["commit", "-m", request.title.as_str()];
    if !request.description.trim().is_empty() {
        args.extend(["-m", request.description.as_str()]);
    }
    let author_email = author_email(&request.author_name);
    git_with_env(
        &repo_dir,
        &[
            ("GIT_AUTHOR_NAME", request.author_name.as_str()),
            ("GIT_AUTHOR_EMAIL", author_email.as_str()),
            ("GIT_COMMITTER_NAME", request.author_name.as_str()),
            ("GIT_COMMITTER_EMAIL", author_email.as_str()),
        ],
        &args,
    )?;

    let commits = list_commits(root, repository)?;
    commits
        .into_iter()
        .find(|commit| commit.branch_name == request.branch_name)
        .context("created commit could not be reloaded")
}

pub fn list_files(
    root: &Path,
    repository_id: uuid::Uuid,
    branch_name: &str,
) -> Result<Vec<RepositoryFile>> {
    let repo_dir = repo_dir(root, repository_id)?;
    let paths = git(&repo_dir, &["ls-tree", "-r", "--name-only", branch_name])?;

    let mut files = Vec::new();
    for path in paths.lines().filter(|line| !line.trim().is_empty()) {
        let content = git(&repo_dir, &["show", &format!("{branch_name}:{path}")])?;
        let last_commit_sha = git(
            &repo_dir,
            &["log", "-n", "1", "--format=%H", branch_name, "--", path],
        )?;
        files.push(RepositoryFile {
            id: uuid::Uuid::now_v7(),
            repository_id,
            path: path.to_string(),
            branch_name: branch_name.to_string(),
            language: infer_language(path).to_string(),
            size_bytes: content.len() as i32,
            content,
            last_commit_sha: last_commit_sha.trim().to_string(),
        });
    }

    Ok(files)
}

pub fn repository_diff(
    root: &Path,
    repository_id: uuid::Uuid,
    default_branch: &str,
    branch_name: &str,
) -> Result<String> {
    let repo_dir = repo_dir(root, repository_id)?;
    if branch_name == default_branch {
        let has_parent = git(&repo_dir, &["rev-list", "--count", branch_name])?
            .trim()
            .parse::<usize>()
            .unwrap_or_default()
            > 1;

        if has_parent {
            return git(
                &repo_dir,
                &["diff", &format!("{branch_name}~1"), branch_name],
            );
        }
        return git(&repo_dir, &["show", "--format=", branch_name]);
    }

    let merge_base = git(&repo_dir, &["merge-base", default_branch, branch_name])?;
    git(&repo_dir, &["diff", merge_base.trim(), branch_name])
}

pub fn run_ci_for_repository(
    root: &Path,
    repository: &RepositoryDefinition,
    branch_name: &str,
) -> Result<CiRun> {
    run_ci_for_repository_with_trigger(root, repository, branch_name, "manual")
}

pub fn run_ci_for_repository_with_trigger(
    root: &Path,
    repository: &RepositoryDefinition,
    branch_name: &str,
    trigger: &str,
) -> Result<CiRun> {
    let repo_dir = repo_dir(root, repository.id)?;
    git(&repo_dir, &["checkout", branch_name])?;
    let commit_sha = git(&repo_dir, &["rev-parse", "HEAD"])?;

    let started_at = Utc::now();
    let commands = ci_commands(&repo_dir, repository.package_kind);
    let mut checks = Vec::new();
    let mut status = "passed".to_string();

    for command in commands {
        let output = match run_command(Some(&repo_dir), command.program, &command.args) {
            Ok(stdout) => format_check_result(command.label, "passed", stdout.trim()),
            Err(error) => {
                status = "failed".to_string();
                format_check_result(command.label, "failed", &error.to_string())
            }
        };
        checks.push(output);
        if status == "failed" {
            break;
        }
    }

    Ok(CiRun {
        id: uuid::Uuid::now_v7(),
        repository_id: repository.id,
        branch_name: branch_name.to_string(),
        commit_sha: commit_sha.trim().to_string(),
        pipeline_name: "package-validation".to_string(),
        status,
        trigger: trigger.to_string(),
        started_at,
        completed_at: Some(Utc::now()),
        checks,
    })
}

pub fn branch_head_sha(
    root: &Path,
    repository_id: uuid::Uuid,
    branch_name: &str,
) -> Result<String> {
    let repo_dir = repo_dir(root, repository_id)?;
    ensure_branch_exists(&repo_dir, branch_name)?;
    let sha = git(
        &repo_dir,
        &["rev-parse", &format!("refs/heads/{branch_name}")],
    )?;
    Ok(sha.trim().to_string())
}

pub fn merge_branches(
    root: &Path,
    repository: &RepositoryDefinition,
    source_branch: &str,
    target_branch: &str,
    author_name: &str,
) -> Result<String> {
    if source_branch == target_branch {
        bail!("source and target branches must be different");
    }

    let repo_dir = repo_dir(root, repository.id)?;
    ensure_branch_exists(&repo_dir, source_branch)?;
    ensure_branch_exists(&repo_dir, target_branch)?;

    let ahead_by = count_ahead_by(&repo_dir, target_branch, source_branch)?;
    if ahead_by <= 0 {
        bail!("branch {source_branch} has no new commits to merge into {target_branch}");
    }

    git(&repo_dir, &["checkout", target_branch])?;
    let author_email = author_email(author_name);
    git_with_env(
        &repo_dir,
        &[
            ("GIT_AUTHOR_NAME", author_name),
            ("GIT_AUTHOR_EMAIL", author_email.as_str()),
            ("GIT_COMMITTER_NAME", author_name),
            ("GIT_COMMITTER_EMAIL", author_email.as_str()),
        ],
        &["merge", "--no-ff", "--no-edit", source_branch],
    )?;

    let sha = git(&repo_dir, &["rev-parse", "HEAD"])?;
    Ok(sha.trim().to_string())
}

fn repo_dir(root: &Path, repository_id: uuid::Uuid) -> Result<PathBuf> {
    let repo_dir = repository_path(root, repository_id);
    if !repo_dir.join(".git").exists() {
        bail!(
            "repository does not exist on disk at {}",
            repo_dir.display()
        );
    }
    Ok(repo_dir)
}

fn scaffold_files(repository: &RepositoryDefinition) -> Vec<(String, String)> {
    match repository_runtime(repository) {
        "typescript-react" => scaffold_typescript_react_files(repository),
        "python" => scaffold_python_files(repository),
        _ => scaffold_rust_files(repository),
    }
}

fn scaffold_rust_files(repository: &RepositoryDefinition) -> Vec<(String, String)> {
    let package_name = repository.slug.replace('-', "_");
    let cargo_toml = format!(
        "[package]\nname = \"{}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n[lib]\npath = \"src/lib.rs\"\n",
        repository.slug
    );
    let lib_rs = format!(
        "pub fn package_name() -> &'static str {{\n    \"{}\"\n}}\n\npub fn package_kind() -> &'static str {{\n    \"{}\"\n}}\n",
        package_name, repository.package_kind
    );
    let readme = format!(
        "# {}\n\n{}\n\nThis repository is managed by OpenFoundry code-repo-service and backed by a real Git repository.\n",
        repository.name,
        if repository.description.trim().is_empty() {
            "OpenFoundry package scaffold.".to_string()
        } else {
            repository.description.clone()
        }
    );
    let manifest = format!(
        "[package]\nname = \"{}\"\nkind = \"{}\"\ndefault_branch = \"{}\"\nowner = \"{}\"\n",
        repository.slug,
        repository.package_kind.as_str(),
        repository.default_branch,
        repository.owner
    );

    vec![
        ("Cargo.toml".to_string(), cargo_toml),
        ("README.md".to_string(), readme),
        ("openfoundry.toml".to_string(), manifest),
        ("src/lib.rs".to_string(), lib_rs),
    ]
}

fn scaffold_typescript_react_files(repository: &RepositoryDefinition) -> Vec<(String, String)> {
    let package_json = serde_json::to_string_pretty(&serde_json::json!({
        "name": repository.slug,
        "version": "0.1.0",
        "private": true,
        "type": "module",
        "scripts": {
            "dev": "vite",
            "build": "tsc -p . && vite build",
            "check": "tsc -p . --noEmit"
        },
        "dependencies": {
            "@open-foundry/sdk": "^0.1.0",
            "react": "^18.3.0",
            "react-dom": "^18.3.0"
        },
        "devDependencies": {
            "@types/react": "^18.3.0",
            "@types/react-dom": "^18.3.0",
            "typescript": "^5.6.0",
            "vite": "^5.4.0"
        }
    }))
    .unwrap_or_else(|_| "{}".to_string());
    let tsconfig = serde_json::to_string_pretty(&serde_json::json!({
        "compilerOptions": {
            "target": "ES2022",
            "module": "ES2022",
            "moduleResolution": "Bundler",
            "jsx": "react-jsx",
            "strict": true,
            "lib": ["ES2022", "DOM"],
            "types": ["vite/client"]
        },
        "include": ["src/**/*.ts", "src/**/*.tsx"]
    }))
    .unwrap_or_else(|_| "{}".to_string());
    let platform_tsx = "import type { OpenFoundryClientOptions } from '@open-foundry/sdk';\n\nexport function platformOptions(): OpenFoundryClientOptions {\n  return {\n    baseUrl: import.meta.env.VITE_OPENFOUNDRY_BASE_URL ?? 'http://127.0.0.1:8080',\n    headers: import.meta.env.VITE_OPENFOUNDRY_TOKEN\n      ? { authorization: `Bearer ${import.meta.env.VITE_OPENFOUNDRY_TOKEN}` }\n      : undefined,\n  };\n}\n".to_string();
    let workspace_description = if repository.description.trim().is_empty() {
        "TypeScript + React starter powered by the OpenFoundry SDK."
    } else {
        &repository.description
    };
    let workspace_tsx = format!(
        "import React from 'react';\nimport {{ useOpenFoundry, useOpenFoundryQuery }} from '@open-foundry/sdk/react';\n\nexport function OperationsConsole() {{\n  const client = useOpenFoundry();\n  const datasets = useOpenFoundryQuery(() => client.datasetDatasetListdatasets(), [client]);\n  const datasetCount = datasets.data?.datasets?.length ?? 0;\n\n  return (\n    <section style={{{{ display: 'grid', gap: 16, gridTemplateColumns: '1fr 1fr', marginTop: 24 }}}}>\n      <article style={{{{ borderRadius: 24, padding: 20, background: '#ffffff', border: '1px solid rgba(148, 163, 184, 0.24)' }}}}>\n        <div style={{{{ fontSize: 12, letterSpacing: '0.18em', textTransform: 'uppercase', opacity: 0.55 }}}}>Workspace</div>\n        <h2 style={{{{ margin: '10px 0 0', fontSize: 28 }}}}>{name}</h2>\n        <p style={{{{ marginTop: 12, opacity: 0.72 }}}}>{description}</p>\n      </article>\n      <article style={{{{ borderRadius: 24, padding: 20, background: '#0f172a', color: '#e2e8f0' }}}}>\n        <div style={{{{ fontSize: 12, letterSpacing: '0.18em', textTransform: 'uppercase', opacity: 0.68 }}}}>Platform signal</div>\n        <div style={{{{ marginTop: 10, fontSize: 40, fontWeight: 700 }}}}>{{datasetCount}}</div>\n        <div style={{{{ marginTop: 6, opacity: 0.72 }}}}>dataset(s) visible from OpenFoundry</div>\n      </article>\n    </section>\n  );\n}}\n",
        name = repository.name,
        description = workspace_description,
    );
    let app_tsx = format!(
        "import React from 'react';\n\
import {{ useOpenFoundry }} from '@open-foundry/sdk/react';\n\
import {{ OperationsConsole }} from './workspaces/OperationsConsole';\n\
\n\
export default function App() {{\n\
  const client = useOpenFoundry();\n\
\n\
  return (\n\
    <main style={{{{ fontFamily: 'Manrope, system-ui, sans-serif', padding: 32, minHeight: '100vh', background: 'linear-gradient(135deg, #f8fafc, #eef6ff)' }}}}>\n\
      <div style={{{{ display: 'flex', justifyContent: 'space-between', gap: 20, flexWrap: 'wrap' }}}}>\n\
        <div>\n\
          <div style={{{{ fontSize: 12, letterSpacing: '0.18em', textTransform: 'uppercase', opacity: 0.55 }}}}>Slate-ready React starter</div>\n\
          <h1>{name}</h1>\n\
          <p>{description}</p>\n\
        </div>\n\
        <div style={{{{ alignSelf: 'flex-start', borderRadius: 999, padding: '10px 16px', background: '#ffffff', border: '1px solid rgba(148, 163, 184, 0.24)' }}}}>Client wired: {{String(Boolean(client))}}</div>\n\
      </div>\n\
      <OperationsConsole />\n\
    </main>\n\
  );\n\
}}\n",
        name = repository.name,
        description = if repository.description.trim().is_empty() {
            "React starter powered by the OpenFoundry SDK."
        } else {
            &repository.description
        }
    );
    let main_tsx = "import React from 'react';\nimport ReactDOM from 'react-dom/client';\nimport { OpenFoundryProvider } from '@open-foundry/sdk/react';\nimport App from './App';\nimport { platformOptions } from './platform';\n\nReactDOM.createRoot(document.getElementById('root')!).render(\n  <React.StrictMode>\n    <OpenFoundryProvider options={platformOptions()}>\n      <App />\n    </OpenFoundryProvider>\n  </React.StrictMode>,\n);\n".to_string();
    let index_html = format!(
        "<!doctype html>\n<html lang=\"en\">\n  <head>\n    <meta charset=\"UTF-8\" />\n    <meta name=\"viewport\" content=\"width=device-width, initial-scale=1.0\" />\n    <title>{}</title>\n  </head>\n  <body>\n    <div id=\"root\"></div>\n    <script type=\"module\" src=\"/src/main.tsx\"></script>\n  </body>\n</html>\n",
        repository.name
    );
    let manifest = format!(
        "[package]\nname = \"{}\"\nkind = \"{}\"\ndefault_branch = \"{}\"\nowner = \"{}\"\nruntime = \"typescript-react\"\nentry_file = \"src/App.tsx\"\ndev_command = \"pnpm dev\"\npreview_command = \"pnpm build\"\n",
        repository.slug,
        repository.package_kind.as_str(),
        repository.default_branch,
        repository.owner
    );
    let readme = format!(
        "# {}\n\n{}\n\nGenerated by OpenFoundry code-repo-service with a TypeScript + React starter wired for `@open-foundry/sdk/react`.\n\n## Workspace\n\n- `src/platform.tsx` centralizes platform connectivity.\n- `src/workspaces/OperationsConsole.tsx` is a richer starter surface ready for Slate-style iteration.\n- `.env.example` documents local platform variables.\n",
        repository.name,
        if repository.description.trim().is_empty() {
            "React starter scaffold.".to_string()
        } else {
            repository.description.clone()
        }
    );
    let env_example =
        "VITE_OPENFOUNDRY_BASE_URL=http://127.0.0.1:8080\nVITE_OPENFOUNDRY_TOKEN=\n".to_string();

    vec![
        ("package.json".to_string(), package_json),
        ("tsconfig.json".to_string(), tsconfig),
        ("index.html".to_string(), index_html),
        (".env.example".to_string(), env_example),
        ("README.md".to_string(), readme),
        ("openfoundry.toml".to_string(), manifest),
        ("src/App.tsx".to_string(), app_tsx),
        ("src/platform.tsx".to_string(), platform_tsx),
        ("src/main.tsx".to_string(), main_tsx),
        (
            "src/workspaces/OperationsConsole.tsx".to_string(),
            workspace_tsx,
        ),
    ]
}

fn scaffold_python_files(repository: &RepositoryDefinition) -> Vec<(String, String)> {
    let module_name = repository.slug.replace('-', "_");
    let pyproject = format!(
        "[project]\nname = \"{}\"\nversion = \"0.1.0\"\ndescription = \"{}\"\nrequires-python = \">=3.11\"\n\n[project.scripts]\n{} = \"{}.cli:main\"\n\n[tool.openfoundry]\nkind = \"{}\"\nruntime = \"python\"\n",
        repository.slug,
        if repository.description.trim().is_empty() {
            "Python starter scaffold"
        } else {
            &repository.description
        },
        module_name,
        module_name,
        repository.package_kind.as_str()
    );
    let module_init = format!(
        "def package_name() -> str:\n    return {:?}\n\n\ndef package_kind() -> str:\n    return {:?}\n",
        repository.slug,
        repository.package_kind.as_str()
    );
    let config_py = "from dataclasses import dataclass\nimport os\n\n\n@dataclass(frozen=True)\nclass PlatformConfig:\n    base_url: str\n    token: str | None\n\n\ndef load_config() -> PlatformConfig:\n    return PlatformConfig(\n        base_url=os.getenv('OPENFOUNDRY_BASE_URL', 'http://127.0.0.1:8080'),\n        token=os.getenv('OPENFOUNDRY_TOKEN'),\n    )\n".to_string();
    let workbench_py = "from .config import load_config\n\n\ndef workspace_summary() -> str:\n    config = load_config()\n    auth_state = 'token' if config.token else 'anonymous'\n    return f'workspace ready against {config.base_url} ({auth_state})'\n".to_string();
    let cli_py = "from . import package_kind, package_name\nfrom .workbench import workspace_summary\n\n\ndef main() -> None:\n    print(f\"{package_name()} ({package_kind()})\")\n    print(workspace_summary())\n\n\nif __name__ == '__main__':\n    main()\n".to_string();
    let main_py = "from .cli import main\n\n\nif __name__ == '__main__':\n    main()\n".to_string();
    let test_py = format!(
        "from {}.workbench import workspace_summary\n\n\ndef test_workspace_summary_mentions_workspace() -> None:\n    assert 'workspace' in workspace_summary()\n",
        module_name
    );
    let manifest = format!(
        "[package]\nname = \"{}\"\nkind = \"{}\"\ndefault_branch = \"{}\"\nowner = \"{}\"\nruntime = \"python\"\nentry_file = \"src/{}/main.py\"\ndev_command = \"python -m {}\"\npreview_command = \"python -m compileall src\"\n",
        repository.slug,
        repository.package_kind.as_str(),
        repository.default_branch,
        repository.owner,
        module_name,
        module_name
    );
    let readme = format!(
        "# {}\n\n{}\n\nGenerated by OpenFoundry code-repo-service with a Python starter package.\n\n## Workspace\n\n- `src/{}` contains config, CLI, and workspace helpers.\n- `tests/test_smoke.py` gives you a first extension point.\n- `.env.example` documents the platform variables.\n",
        repository.name,
        if repository.description.trim().is_empty() {
            "Python starter scaffold.".to_string()
        } else {
            repository.description.clone()
        },
        module_name
    );
    let env_example =
        "OPENFOUNDRY_BASE_URL=http://127.0.0.1:8080\nOPENFOUNDRY_TOKEN=\n".to_string();

    vec![
        ("pyproject.toml".to_string(), pyproject),
        (".env.example".to_string(), env_example),
        ("README.md".to_string(), readme),
        ("openfoundry.toml".to_string(), manifest),
        (format!("src/{module_name}/__init__.py"), module_init),
        (format!("src/{module_name}/config.py"), config_py),
        (format!("src/{module_name}/workbench.py"), workbench_py),
        (format!("src/{module_name}/cli.py"), cli_py),
        (format!("src/{module_name}/__main__.py"), main_py.clone()),
        (format!("src/{module_name}/main.py"), main_py),
        ("tests/test_smoke.py".to_string(), test_py),
    ]
}

fn repository_runtime(repository: &RepositoryDefinition) -> &str {
    repository
        .settings
        .get("runtime")
        .and_then(|value| value.as_str())
        .unwrap_or("rust")
}

fn count_ahead_by(repo_dir: &Path, base_branch: &str, branch_name: &str) -> Result<i32> {
    let output = git(
        repo_dir,
        &[
            "rev-list",
            "--count",
            &format!("{base_branch}..{branch_name}"),
        ],
    )?;
    Ok(output.trim().parse::<i32>().unwrap_or_default())
}

fn ensure_branch_exists(repo_dir: &Path, branch_name: &str) -> Result<()> {
    git(
        repo_dir,
        &[
            "rev-parse",
            "--verify",
            &format!("refs/heads/{branch_name}"),
        ],
    )?;
    Ok(())
}

fn parse_git_timestamp(raw: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .with_context(|| format!("failed to parse git timestamp '{raw}'"))
        .map(|value| value.with_timezone(&Utc))
}

fn write_repo_file(repo_dir: &Path, relative_path: &str, content: &str) -> Result<()> {
    let file_path = repo_dir.join(relative_path);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create parent directory {}", parent.display()))?;
    }
    fs::write(&file_path, content)
        .with_context(|| format!("failed to write {}", file_path.display()))
}

fn author_email(author_name: &str) -> String {
    format!(
        "{}@openfoundry.dev",
        author_name.trim().to_lowercase().replace(' ', ".")
    )
}

fn infer_language(path: &str) -> &'static str {
    match path.rsplit('.').next().unwrap_or_default() {
        "md" => "markdown",
        "rs" => "rust",
        "toml" => "toml",
        "json" => "json",
        "ts" => "typescript",
        "tsx" => "typescript",
        "js" => "javascript",
        "py" => "python",
        "yml" | "yaml" => "yaml",
        _ => "text",
    }
}

fn slug_fragment(value: &str) -> String {
    let mut fragment = String::new();
    for character in value.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            fragment.push(character);
        } else if !fragment.ends_with('-') {
            fragment.push('-');
        }
    }
    fragment.trim_matches('-').to_string()
}

fn git(repo_dir: &Path, args: &[&str]) -> Result<String> {
    run_command(Some(repo_dir), "git", args)
}

fn git_with_env(repo_dir: &Path, envs: &[(&str, &str)], args: &[&str]) -> Result<String> {
    run_command_with_env(Some(repo_dir), "git", args, envs)
}

fn run_command(current_dir: Option<&Path>, program: &str, args: &[&str]) -> Result<String> {
    run_command_with_env(current_dir, program, args, &[])
}

fn run_command_with_env(
    current_dir: Option<&Path>,
    program: &str,
    args: &[&str],
    envs: &[(&str, &str)],
) -> Result<String> {
    let mut command = Command::new(program);
    command.args(args);
    if let Some(current_dir) = current_dir {
        command.current_dir(current_dir);
    }
    for (name, value) in envs {
        command.env(name, value);
    }

    let output = command
        .output()
        .with_context(|| format!("failed to run command `{program} {}`", args.join(" ")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        bail!(
            "command `{program} {}` failed: {}{}{}",
            args.join(" "),
            stderr.trim(),
            if stderr.trim().is_empty() || stdout.trim().is_empty() {
                ""
            } else {
                " | "
            },
            stdout.trim()
        );
    }

    String::from_utf8(output.stdout).context("command output was not valid UTF-8")
}

struct CiCommand {
    label: &'static str,
    program: &'static str,
    args: Vec<&'static str>,
}

fn ci_commands(repo_dir: &Path, package_kind: PackageKind) -> Vec<CiCommand> {
    if repo_dir.join("Cargo.toml").exists() {
        return vec![
            CiCommand {
                label: "cargo check",
                program: "cargo",
                args: vec!["check", "--offline", "--quiet"],
            },
            CiCommand {
                label: "cargo test",
                program: "cargo",
                args: vec!["test", "--offline", "--quiet"],
            },
        ];
    }

    if repo_dir.join("pyproject.toml").exists() {
        return vec![CiCommand {
            label: "python compileall",
            program: "python3",
            args: vec!["-m", "compileall", "src"],
        }];
    }

    match package_kind {
        PackageKind::AiAgent | PackageKind::MlModel => vec![CiCommand {
            label: "python compileall",
            program: "python3",
            args: vec!["-m", "compileall", "."],
        }],
        _ => vec![
            CiCommand {
                label: "git fsck",
                program: "git",
                args: vec!["fsck", "--no-progress"],
            },
            CiCommand {
                label: "git diff --check",
                program: "git",
                args: vec!["diff", "--check"],
            },
        ],
    }
}

fn format_check_result(label: &str, status: &str, details: &str) -> String {
    if details.is_empty() {
        format!("{label}: {status}")
    } else {
        format!("{label}: {status} ({})", details.replace('\n', " | "))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{
        commit::{CommitFileChange, CreateCommitRequest},
        repository::{PackageKind, RepositoryDefinition, RepositoryVisibility},
    };
    use serde_json::json;

    fn temp_root() -> PathBuf {
        let root =
            std::env::temp_dir().join(format!("of-code-repo-tests-{}", uuid::Uuid::now_v7()));
        fs::create_dir_all(&root).expect("temp root");
        root
    }

    fn sample_repository() -> RepositoryDefinition {
        RepositoryDefinition {
            id: uuid::Uuid::now_v7(),
            name: "Smoke Repo".to_string(),
            slug: "smoke-repo".to_string(),
            description: "Real Git-backed repo".to_string(),
            owner: "Platform UI".to_string(),
            default_branch: "main".to_string(),
            visibility: RepositoryVisibility::Private,
            object_store_backend: "local".to_string(),
            package_kind: PackageKind::Widget,
            tags: vec!["smoke".to_string()],
            settings: json!({}),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    fn sample_repository_with_runtime(
        runtime: &str,
        package_kind: PackageKind,
    ) -> RepositoryDefinition {
        RepositoryDefinition {
            package_kind,
            settings: json!({ "runtime": runtime }),
            ..sample_repository()
        }
    }

    #[test]
    fn initializes_repository_with_real_git_history() {
        let root = temp_root();
        let repository = sample_repository();

        let (head_sha, files) = initialize_repository(&root, &repository).expect("init repo");

        assert!(!head_sha.is_empty());
        assert!(repository_path(&root, repository.id).join(".git").exists());
        assert!(files.iter().any(|file| file.path == "Cargo.toml"));
    }

    #[test]
    fn creates_branches_commits_and_diff_against_default_branch() {
        let root = temp_root();
        let repository = sample_repository();
        initialize_repository(&root, &repository).expect("init repo");
        create_branch(&root, repository.id, "feature/runtime", "main").expect("create branch");

        let commit = apply_commit(
            &root,
            &repository,
            &CreateCommitRequest {
                branch_name: "feature/runtime".to_string(),
                title: "Add runtime helper".to_string(),
                description: "Adds a real file change".to_string(),
                author_name: "Platform UI".to_string(),
                additions: 8,
                deletions: 0,
                files: vec![CommitFileChange {
                    path: "src/runtime.rs".to_string(),
                    content: "pub fn runtime_ready() -> bool {\n    true\n}\n".to_string(),
                    delete: false,
                }],
            },
        )
        .expect("commit");

        let commits = list_commits(&root, &repository).expect("list commits");
        let diff = repository_diff(&root, repository.id, "main", "feature/runtime").expect("diff");

        assert_eq!(commit.branch_name, "feature/runtime");
        assert!(commits.iter().any(|entry| entry.sha == commit.sha));
        assert!(diff.contains("src/runtime.rs"));
    }

    #[test]
    fn executes_real_ci_commands_against_repository_worktree() {
        let root = temp_root();
        let repository = sample_repository();
        initialize_repository(&root, &repository).expect("init repo");

        let run = run_ci_for_repository(&root, &repository, "main").expect("ci run");

        assert_eq!(run.status, "passed");
        assert!(!run.commit_sha.is_empty());
        assert!(run.checks.iter().any(|check| check.contains("cargo check")));
    }

    #[test]
    fn merges_feature_branch_back_into_target_branch() {
        let root = temp_root();
        let repository = sample_repository();
        initialize_repository(&root, &repository).expect("init repo");
        create_branch(&root, repository.id, "feature/runtime", "main").expect("create branch");
        apply_commit(
            &root,
            &repository,
            &CreateCommitRequest {
                branch_name: "feature/runtime".to_string(),
                title: "Add runtime helper".to_string(),
                description: "Adds a real file change".to_string(),
                author_name: "Platform UI".to_string(),
                additions: 8,
                deletions: 0,
                files: vec![CommitFileChange {
                    path: "src/runtime.rs".to_string(),
                    content: "pub fn runtime_ready() -> bool {\n    true\n}\n".to_string(),
                    delete: false,
                }],
            },
        )
        .expect("commit");

        let merge_sha =
            merge_branches(&root, &repository, "feature/runtime", "main", "Platform UI")
                .expect("merge");
        let files = list_files(&root, repository.id, "main").expect("files");

        assert!(!merge_sha.is_empty());
        assert!(files.iter().any(|file| file.path == "src/runtime.rs"));
    }

    #[test]
    fn ci_trigger_can_be_overridden_for_push_and_merge_flows() {
        let root = temp_root();
        let repository = sample_repository();
        initialize_repository(&root, &repository).expect("init repo");

        let run =
            run_ci_for_repository_with_trigger(&root, &repository, "main", "push").expect("ci run");

        assert_eq!(run.trigger, "push");
    }

    #[test]
    fn scaffolds_typescript_react_when_requested() {
        let root = temp_root();
        let repository = sample_repository_with_runtime("typescript-react", PackageKind::Widget);

        let (_, files) = initialize_repository(&root, &repository).expect("init repo");

        assert!(files.iter().any(|file| file.path == "package.json"));
        assert!(files.iter().any(|file| file.path == "src/App.tsx"));
        assert!(files.iter().any(|file| file.path == "src/platform.tsx"));
        assert!(
            files
                .iter()
                .any(|file| file.path == "src/workspaces/OperationsConsole.tsx")
        );
        assert!(files.iter().any(|file| file.path == "index.html"));
    }

    #[test]
    fn scaffolds_python_when_requested() {
        let root = temp_root();
        let repository = sample_repository_with_runtime("python", PackageKind::AiAgent);

        let (_, files) = initialize_repository(&root, &repository).expect("init repo");

        assert!(files.iter().any(|file| file.path == "pyproject.toml"));
        assert!(files.iter().any(|file| file.path.ends_with("__init__.py")));
        assert!(files.iter().any(|file| file.path.ends_with("config.py")));
        assert!(files.iter().any(|file| file.path == "tests/test_smoke.py"));
    }
}
