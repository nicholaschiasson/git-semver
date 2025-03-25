use std::{
    char,
    collections::HashMap,
    error,
    fmt::{Debug, Display},
};

use git2::{IntoCString, Oid, Reference};
use regex::Regex;
use semver_extra::{semver::Version, Increment, IncrementLevel};

use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version)]
/// Generate a semantic versioning compliant tag for your HEAD commit.
struct Cli {
    /// The name of your repository's main branch. Useful if you continue to use "master" or "trunk".
    #[arg(short, long, default_value = "main")]
    main_branch: String,

    /// Identifier to use for prerelease during non-main branch execution, using branch name slug when omitted.
    #[arg(short, long)]
    prerelease_id: Option<String>,

    /// Revision to use for prerelease during non-main branch execution, using short commit hash when omitted.
    #[arg(short = 'r', long)]
    prerelease_revision: Option<String>,

    /// Explicit increment level override for use during main branch execution, forcing to ignore the increment level derived from commit summary.
    #[arg(short, long)]
    increment: Option<IncrementLevel>,

    /// Increment level override for non-merge commits to main branch, ie. commits directly to main branch.
    #[arg(long, default_value_t = IncrementLevel::Patch)]
    default_increment: IncrementLevel,

    /// Regular expression to match the increment level in the commit summary of a commit to the main branch.
    #[arg(
        short = 'e',
        long,
        default_value = r"^Merge .*(patch|minor|major)/[\w-]+"
    )]
    match_expression: String,
}

#[derive(Clone, Copy)]
enum Error {
    CommitSummaryWithoutIncrementLevel,
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Error(\"")?;
        Display::fmt(self, f)?;
        f.write_str("\")")?;
        Ok(())
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::CommitSummaryWithoutIncrementLevel => {
                f.write_str("cannot derive version increment level from commit summary")
            }
        }
    }
}

impl error::Error for Error {}

fn main() -> Result<(), Box<dyn error::Error>> {
    let cli = Cli::parse();

    git2::Config::open_default()?.set_str("safe.directory", "*")?;

    let repository = git2::Repository::open_from_env()?;

    let head = repository.head()?;

    let head_commit = head.peel_to_commit()?;

    let head_shorthand = head.shorthand_bytes().into_c_string()?.into_string()?;

    let head_short_id = head_commit
        .as_object()
        .short_id()?
        .into_c_string()?
        .into_string()?;

    let mut head_is_main = false;

    let mut main_branch = repository
        .find_branch(&cli.main_branch, git2::BranchType::Local)
        .map(|b| b.get().peel_to_commit())?;

    while let Ok(commit) = main_branch {
        if commit.id() == head_commit.id() {
            head_is_main = true;
            break;
        }
        if commit.time() < head_commit.time() {
            break;
        }
        main_branch = commit.parent(0);
    }

    let tags: HashMap<Oid, Version> = repository
        .references()?
        .flatten()
        .filter(Reference::is_tag)
        .filter_map(|reference| {
            let tag_target = reference.peel_to_tag().map(|tag| tag.target_id());
            let target = reference.target();
            let shorthand = reference.shorthand().map(Version::parse);
            match (tag_target, target, shorthand) {
                (Ok(tag_target), Some(target), Some(Ok(shorthand))) => {
                    Some(vec![(tag_target, shorthand.clone()), (target, shorthand)])
                }
                (Ok(tag_target), _, Some(Ok(shorthand))) => Some(vec![(tag_target, shorthand)]),
                (_, Some(target), Some(Ok(shorthand))) => Some(vec![(target, shorthand)]),
                _ => None,
            }
        })
        .flatten()
        .collect();

    let mut tag = Version::new(0, 0, 0);
    let mut head_is_tagged = false;

    let mut tag_commit = Ok(head_commit.clone());

    while let Ok(commit) = tag_commit {
        if let Some(t) = tags.get(&commit.id()) {
            head_is_tagged = commit.id() == head_commit.id();
            tag = t.clone();
            break;
        }
        tag_commit = commit.parent(0);
    }

    if !head_is_tagged {
        let commit_match_expression = Regex::new(cli.match_expression.as_str())?;

        if head_is_main {
            let head_summary_increment = head_commit
                .summary()
                .ok_or(Error::CommitSummaryWithoutIncrementLevel)
                .map(|summary| {
                    commit_match_expression
                        .captures(summary)
                        .ok_or(Error::CommitSummaryWithoutIncrementLevel)
                        .map(|captures| captures[1].parse::<IncrementLevel>())
                });
            let increment_level = match (cli.increment, head_summary_increment) {
                (Some(increment), _) => increment,
                (_, Ok(Ok(Ok(increment)))) => increment,
                _ => cli.default_increment,
            };
            tag.increment(increment_level);
        } else {
            tag.pre = semver_extra::semver::Prerelease::new(&format!(
                "{}.{}",
                slug(&cli.prerelease_id.unwrap_or(head_shorthand)),
                cli.prerelease_revision.unwrap_or(head_short_id)
            ))?;
        }
    }

    println!("{tag}");

    Ok(())
}

fn slug(s: &str) -> String {
    const TEMP_DELIM: char = ' ';
    s.chars()
        .map(|c| if c.is_alphanumeric() { c } else { TEMP_DELIM })
        .collect::<String>()
        .split(TEMP_DELIM)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug() {
        assert_eq!(
            slug("//.hello////42349()*'']-=_+1`~world1----"),
            "hello-42349-1-world1"
        );
    }
}
