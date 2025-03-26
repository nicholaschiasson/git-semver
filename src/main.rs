use std::{char, collections::HashMap, error, fmt::Debug};

use git2::{Commit, IntoCString, Oid, Reference};
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

    /// Skips commits that do not match the expression when incrementing the version from the previous tag.
    /// Useful when not squashing merge commits, but only want to increment on the merge commit.
    /// Note: HEAD commit is never skipped, even if it does not match the expression.
    #[arg(short = 's', long)]
    skip_unmatched_commits: bool,
}

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

    // Walk back from main branch ref until we find HEAD (or not).
    // Determine if we are on the main branch.
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

    // Build a dictionary of object IDs mapping to version tags.
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

    let mut version = Version::new(0, 0, 0);
    let mut head_is_tagged = false;
    let commit_match_expression = Regex::new(cli.match_expression.as_str())?;
    let mut increments = Vec::new();

    let mut tag_commit = Ok(head_commit.clone());

    // Walk back from HEAD until we find the most recent tag.
    // Keep track of all of the version increments that need to be applied.
    while let Ok(commit) = tag_commit {
        if let Some(t) = tags.get(&commit.id()) {
            head_is_tagged = commit.id() == head_commit.id();
            version = t.clone();
            break;
        }
        increments.push(determine_increment_level(
            &commit,
            &commit_match_expression,
            cli.default_increment,
            cli.skip_unmatched_commits,
        ));
        tag_commit = commit.parent(0);
    }

    // Override HEAD increment explicitly, or at least don't allow it to be skipped if unmatched.
    if let Some(i) = increments.first_mut() {
        *i = if let Some(increment) = cli.increment {
            Some(increment)
        } else {
            determine_increment_level(
                &head_commit,
                &commit_match_expression,
                cli.default_increment,
                false,
            )
        }
    };

    // Determine the version
    if !head_is_tagged {
        for &increment_level in increments
            .iter()
            .skip(if head_is_main { 0 } else { 1 })
            .flatten()
            .rev()
        {
            version.increment(increment_level);
        }
        if !head_is_main {
            version.pre = semver_extra::semver::Prerelease::new(&format!(
                "{}.{}",
                slug(&cli.prerelease_id.unwrap_or(head_shorthand)),
                cli.prerelease_revision.unwrap_or(head_short_id)
            ))?;
        }
    }

    println!("{version}");

    Ok(())
}

fn determine_increment_level(
    commit: &Commit,
    commit_match_expression: &Regex,
    default_increment: IncrementLevel,
    skip_unmatched: bool,
) -> Option<IncrementLevel> {
    let commit_summary_increment = commit.summary().map(|summary| {
        commit_match_expression
            .captures(summary)
            .map(|captures| captures[1].parse::<IncrementLevel>())
    });
    match (commit_summary_increment, skip_unmatched) {
        (Some(Some(Ok(increment))), _) => Some(increment),
        (_, false) => Some(default_increment),
        (_, true) => None,
    }
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
