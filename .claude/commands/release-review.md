# Release Version with Code Review

Release a new version of Timesheeter after performing a comprehensive code review.

**Usage:** `/release-review <version>` (e.g., `/release-review 1.4.0`)

## Steps to Perform

### 1. Validate Version Argument
- Ensure a version number is provided as argument: $ARGUMENTS
- Version must follow semver format (e.g., 1.4.0)
- If no version provided, ask the user for the version number

### 2. Perform Full Code Review Since Last Version
Execute the code-review command first:
- Security Review
- Maintainability Review
- Accessibility Review
- Documentation Review
- README verification

**IMPORTANT:** If critical issues are found during the review:
- Present the findings to the user
- Ask if they want to proceed with the release or fix issues first
- Do NOT proceed with release if user wants to fix issues or if there are critical issues

### 3. Get Changes Since Last Version
- Run `git describe --tags --abbrev=0` to get the last tag
- Run `git log --oneline <last-tag>..HEAD` to get commits since last release
- If no commits since last tag, inform user and abort

### 4. Update Version in Cargo.toml
- Update the `version` field in Cargo.toml to the new version
- Verify the change with `cargo check`

### 5. Generate Changelog Entry
- Read the existing CHANGELOG.md
- Create a new entry at the top following the Keep a Changelog format
- Include today's date in YYYY-MM-DD format
- Categorize changes into: Added, Changed, Fixed, Removed (as applicable)
- Reference relevant commit messages and GitHub issues

### 6. Commit Changes
- Stage Cargo.toml and CHANGELOG.md
- Commit with message: `🔖 Release v<version>`
- Include a summary of changes in the commit body
- Add `Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>`

### 7. Create Git Tag
- Create an annotated tag: `git tag -a v<version> -m "Release v<version> - <brief description>"`

### 8. Push to Remote
- Push commits: `git push origin main`
- Push tags: `git push origin --tags`

### 9. Summary
- Display the code review summary
- Display the new version number
- Show the changelog entry that was added
- Confirm successful push
