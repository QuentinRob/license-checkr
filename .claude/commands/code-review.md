# Code Review

Perform a code review of changes between versions.

**Usage:** `/code-review [from-version] [to-version]`

**Examples:**
- `/code-review` - Review changes since the last tag to HEAD
- `/code-review v1.3.0` - Review changes from v1.3.0 to HEAD
- `/code-review v1.3.0 v1.4.0` - Review changes between v1.3.0 and v1.4.0

## Steps to Perform

### 1. Determine Version Range
- If no arguments: use `git describe --tags --abbrev=0` to get the last tag, compare to HEAD
- If one argument: use that tag as the starting point, compare to HEAD
- If two arguments: compare between the two specified tags
- Run `git log --oneline <from>..<to>` to list commits in range
- Run `git diff --stat <from>..<to>` to get overview of changed files

### 2. Get Changed Files
- Run `git diff --name-only <from>..<to>` to get list of modified files
- Categorize files by type (handlers, models, templates, migrations, etc.)

### 3. Review Focus Areas

For each changed file, review the following based on file type:

#### Rust Code (src/)
- **Security**: Input validation, SQL injection prevention, authentication checks
- **Error Handling**: Proper error propagation, user-friendly messages
- **Code Quality**: Naming conventions, DRY principle, complexity
- **RBAC**: Permission checks on new/modified endpoints

#### Templates (templates/)
- **CSRF Protection**: All forms include csrf_token
- **Accessibility**: ARIA attributes, keyboard navigation, focus management
- **XSS Prevention**: Proper escaping of user data
- **Permission Conditionals**: UI elements respect user_permissions
- **Code duplication**: Avoid repetition, use macros or components

#### Migrations (migrations/)
- **Schema Safety**: No destructive changes without migration path
- **Indexes**: Appropriate indexes for new tables/columns
- **Constraints**: Foreign keys, unique constraints, check constraints

#### CSS (static/css/)
- **CSS Variables**: Use design tokens, no hardcoded colors
- **Responsive**: Mobile breakpoints considered
- **Dark Mode**: Theme compatibility

### 4. Security Checklist
For the changed code, verify:
- [ ] No SQL injection vulnerabilities (Diesel ORM used)
- [ ] CSRF tokens on all forms
- [ ] Permission checks on protected routes
- [ ] Input validation on user data
- [ ] No sensitive data in logs or responses
- [ ] Proper error handling (no stack traces exposed)

### 5. Accessibility Checklist
For UI changes, verify:
- [ ] ARIA labels on interactive elements
- [ ] Keyboard navigation works
- [ ] Focus indicators visible
- [ ] Form labels associated with inputs
- [ ] Screen reader announcements for dynamic content

### 6. Create GitHub Issues for Findings

**All issues found during the review MUST be tracked via GitHub issues.**

For each issue found:
1. **Create a GitHub issue** using `gh issue create`
2. **Apply appropriate labels**:
   - `bug` - For security vulnerabilities, broken functionality
   - `enhancement` - For code quality improvements
   - `ui` - For accessibility or UI issues
   - `refactor` - For code structure improvements
   - `security` - For security-related findings (create this label if needed)
3. **Set priority in title** using prefix:
   - `[CRITICAL]` - Security vulnerabilities, data loss risks
   - `[HIGH]` - Broken functionality, major accessibility gaps
   - `[MEDIUM]` - Code quality issues, minor bugs
   - `[LOW]` - Nice-to-have improvements
4. **Include in issue body**:
   - Description of the issue
   - File(s) and line number(s) affected
   - Suggested fix or approach
   - Reference to the code review (version range reviewed)

**Example:**
```bash
gh issue create \
  --title "[HIGH] Missing RBAC permission check on /workload endpoint" \
  --label "bug,security" \
  --body "## Description
Missing permission check in workload handler.

## Location
- File: src/handlers/workload_schedules.rs
- Line: 45

## Suggested Fix
Add \`require_permission(&pool, &req, \"workload\", \"view\")\` check.

## Found In
Code review of v1.3.6..v1.4.0"
```

### 7. Track Issue Resolution

After issues are created:
- List all created issue numbers in the review summary
- When fixing issues, reference the issue number in commits
- Close issues with a comment describing the fix and files modified

---

## Output Format

### Summary
Brief overview of what changed in this version range:
- Number of commits
- Files changed (by category)
- Key features/fixes included

### Issues Found

#### Critical Issues
Security vulnerabilities or bugs that need immediate attention.
- **Issue #XX**: [Title] - Brief description

#### High Priority
Functionality issues, major accessibility gaps.
- **Issue #XX**: [Title] - Brief description

#### Medium Priority
Code quality issues, minor improvements.
- **Issue #XX**: [Title] - Brief description

#### Low Priority
Nice-to-have improvements, refactoring suggestions.
- **Issue #XX**: [Title] - Brief description

### Created Issues Summary
List of all GitHub issues created during this review:
```
#XX - [CRITICAL] Issue title
#XX - [HIGH] Issue title
#XX - [MEDIUM] Issue title
```

### Positive Observations
Well-implemented patterns and good practices observed in the changes.

### Next Steps
- Fix critical and high priority issues before next release
- Schedule medium/low priority issues for future sprints
