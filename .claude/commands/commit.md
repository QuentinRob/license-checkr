# Commit with Gitmoji

Create a well-structured commit message using gitmoji conventions, linked to a GitHub issue.

**Usage:** `/commit [issue-number] [--close]`

**Examples:**
- `/commit 42` - Commit referencing issue #42
- `/commit 42 --close` - Commit that closes issue #42
- `/commit` - Will create the related github issue based on the modifications content following the github issue creation rules in @rules/development-workflow.md

## Gitmoji Reference

Use these gitmoji prefixes based on the type of change:

| Emoji | Code | Description |
|-------|------|-------------|
| :sparkles: | `✨` | New feature |
| :bug: | `🐛` | Bug fix |
| :recycle: | `♻️` | Refactor code |
| :lipstick: | `💄` | UI/style updates |
| :zap: | `⚡️` | Performance improvement |
| :lock: | `🔒️` | Security fix |
| :wrench: | `🔧` | Configuration changes |
| :memo: | `📝` | Documentation |
| :white_check_mark: | `✅` | Add/update tests |
| :construction: | `🚧` | Work in progress |
| :arrow_up: | `⬆️` | Upgrade dependencies |
| :fire: | `🔥` | Remove code/files |
| :truck: | `🚚` | Move/rename files |
| :boom: | `💥` | Breaking changes |
| :art: | `🎨` | Improve structure/format |
| :adhesive_bandage: | `🩹` | Simple fix for non-critical issue |
| :building_construction: | `🏗️` | Architectural changes |
| :bookmark: | `🔖` | Release/version tag |
| :rotating_light: | `🚨` | Fix compiler/linter warnings |
| :globe_with_meridians: | `🌐` | Internationalization |
| :wheelchair: | `♿️` | Accessibility improvements |
| :card_file_box: | `🗃️` | Database changes |
| :loud_sound: | `🔊` | Add/update logs |
| :mute: | `🔇` | Remove logs |
| :passport_control: | `🛂` | Authorization/permissions |
| :technologist: | `🧑‍💻` | Developer experience |
| :rewind: | `⏪️` | Revert changes |
| :twisted_rightwards_arrows: | `🔀` | Merge branches |

## Steps to Perform

### 1. Gather Current State
Run these commands to understand what will be committed:

```bash
# Check staged and unstaged changes
git status

# Show detailed diff of staged changes
git diff --staged

# Show detailed diff of unstaged changes (if any)
git diff

# Get recent commit messages for style reference
git log --oneline -10
```

### 2. Verify Issue Reference
If an issue number was provided:
```bash
# View the issue details
gh issue view <issue-number>
```

If no issue number provided:
- List open issues: `gh issue list --state open`
- Ask the user which issue this commit relates to
- **IMPORTANT**: Every commit MUST reference an issue per project workflow

### 3. Analyze Changes
Based on the diff output, determine:
- **Type of change**: feature, fix, refactor, style, etc.
- **Appropriate gitmoji**: Select from the reference table above
- **Scope**: What area of the codebase is affected (handlers, models, templates, etc.)
- **Impact**: What does this change accomplish?

### 4. Stage Files (if needed)
If there are unstaged changes that should be included:
```bash
# Stage specific files (preferred)
git add <file1> <file2>

# Or stage all changes (use cautiously)
git add -A
```

**IMPORTANT**:
- Never stage sensitive files (.env, credentials, etc.)
- Prefer staging specific files over `git add -A`
- Verify staged files with `git status` before committing

### 5. Construct Commit Message

Format:
```
<gitmoji> <Short summary in imperative mood>

<Detailed description of what changed and why>

<Issue reference>

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

**Issue Reference Formats:**
- Reference only: `Refs #42` or `Related to #42`
- Close issue: `Closes #42` or `Fixes #42`

**Guidelines:**
- First line: 50 characters max, imperative mood ("Add" not "Added")
- Body: Wrap at 72 characters
- Explain WHAT changed and WHY, not HOW
- Use bullet points for multiple changes

### 6. Create the Commit

Use a HEREDOC for proper formatting:
```bash
git commit -m "$(cat <<'EOF'
<gitmoji> <Summary>

<Body>

<Issue reference>

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
EOF
)"
```

### 7. Verify Commit
```bash
# Show the commit that was just created
git log -1

# Verify status is clean
git status
```

---

## Example Commits

### Feature Commit
```
✨ Add workload schedule management

Implement CRUD operations for workload schedules allowing users
to define expected hours per project per week.

- Add WorkloadSchedule model with Diesel mappings
- Create handlers for create/read/update/delete
- Add templates with HTMX integration
- Include permission checks for schedule management

Closes #15

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

### Bug Fix Commit
```
🐛 Fix time entry overlap validation

Time entries were incorrectly allowing overlaps when the end time
matched exactly with another entry's start time.

Updated validation logic to use exclusive end time comparison.

Fixes #23

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

### Refactor Commit
```
♻️ Extract permission checking into middleware

Move repeated permission checking logic from individual handlers
into a reusable middleware component for consistency.

- Create PermissionMiddleware struct
- Update handlers to use middleware
- Add tests for permission edge cases

Refs #18

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

### UI/Style Commit
```
💄 Improve dark mode contrast in timesheet grid

Increase contrast ratios for time entry cards in dark mode
to meet WCAG AA accessibility standards.

Refs #31

Co-Authored-By: Claude Opus 4.5 <noreply@anthropic.com>
```

---

## Validation Checklist

Before creating the commit, verify:
- [ ] Changes are staged correctly (`git status`)
- [ ] No sensitive files included (.env, credentials)
- [ ] Issue number is valid and exists
- [ ] Gitmoji matches the type of change
- [ ] Summary is clear and in imperative mood
- [ ] Body explains the "what" and "why"
- [ ] Issue reference format is correct (Refs/Closes/Fixes)

---

## Error Handling

### No Issue Provided
Prompt the user to provide an issue number. Show open issues with:
```bash
gh issue list --state open --limit 20
```

### Pre-commit Hook Failure
If the commit fails due to pre-commit hooks:
1. Fix the issues reported by the hook
2. Stage the fixes: `git add <fixed-files>`
3. Create a NEW commit (do NOT use --amend)
4. The original commit did not happen, so amending would modify the wrong commit

### Nothing to Commit
If `git status` shows no changes:
- Inform the user there are no changes to commit
- Ask if they need to stage changes first
