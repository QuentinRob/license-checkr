# Development Workflow

## Absolute Rule: No Change Without an Issue

**EVERY change to the codebase MUST have a corresponding GitHub issue created BEFORE implementation begins.**

This rule applies to ALL changes without exception:
- Features (large or small)
- Enhancements and improvements
- Bug fixes
- User feedback and requests
- Code review findings
- UI/styling changes
- Refactoring
- Dependency updates
- Configuration changes
- Documentation updates
- Security fixes

**NO EXCEPTIONS.** Even a single-line fix requires an issue.

---

## Core Workflow

### 1. Create Issue First
Before writing ANY code:
1. **Create a GitHub issue** using `gh issue create`
2. **Apply appropriate labels** (see Labels Convention below)
3. **Set priority label**: `priority:critical`, `priority:high`, `priority:medium`, `priority:low`
4. **Include required information** based on issue type (see sections below)

### 2. Implement the Change
1. **Reference the issue** in your work
2. **Update issue status** to `status:in-progress` when starting
3. **Follow acceptance criteria** if defined
4. **Test the change** before considering it complete

### 3. Close Issue Upon Completion
**Every issue MUST be closed with a detailed comment including:**
- What was implemented/fixed
- Files modified/created (with paths)
- Any relevant technical details
- Reference to commits if applicable

**Example closing comment:**
```
Implemented workload schedule feature.

Files modified:
- src/handlers/workload_schedules.rs (new)
- src/models/workload_schedule.rs (new)
- src/main.rs (added routes)
- templates/workload_schedules/index.html (new)
- templates/base.html (added nav link)

Commits: abc1234, def5678
```

---

## Issue Types

### Feature Development

#### 1. Feature Request Intake
When a new feature is requested:
1. **Create a Milestone** - Name it descriptively (e.g., "Workload Schedule")
2. **Create a Feature Label** - Format: `feature:<feature-name>`
3. **Create User Stories** - Break down into implementable stories

#### 2. User Story Format
Each user story issue should include:
```markdown
## User Story
As a [role], I want [feature] so that [benefit].

## Acceptance Criteria
- [ ] Criterion 1
- [ ] Criterion 2
- [ ] Criterion 3

## Technical Notes
- Related files: ...
- Dependencies: #XX

## Priority
[HIGH/MEDIUM/LOW]
```

#### 3. Feature Completion
- Verify all user stories in the milestone are closed
- Close the milestone when fully delivered

---

### User Feedback

**All user feedback MUST be tracked as issues.**

#### When User Provides Feedback:
1. **Create an issue immediately** with `feedback` label
2. **Quote the user's exact feedback** in the issue body
3. **Add appropriate secondary labels** (`ui`, `enhancement`, `bug`, etc.)
4. **Implement** the requested change
5. **Close with details** of what was done

**Issue template:**
```markdown
## User Feedback
> "[Exact user feedback quoted here]"

## Interpretation
[Your understanding of what needs to be done]

## Acceptance Criteria
- [ ] [What will satisfy this feedback]

## Priority
[Based on user's urgency/importance]
```

---

### Bug Reports

#### Bug Issue Format:
```markdown
## Description
What is happening vs. what should happen.

## Steps to Reproduce
1. Step one
2. Step two
3. Step three

## Expected Behavior
What should happen.

## Actual Behavior
What is currently happening.

## Environment
Browser, OS, or other relevant context.

## Priority
[CRITICAL/HIGH/MEDIUM/LOW]
```

#### Bug Fix Closure:
Close with:
- Root cause identified
- How it was fixed
- Files modified
- How to verify the fix

---

### Code Review Findings

**All issues found during code reviews MUST become GitHub issues.**

#### Code Review Issue Format:
```markdown
## Finding
[Description of the issue found]

## Location
- File: [path/to/file.rs]
- Line(s): [line numbers]

## Severity
[CRITICAL/HIGH/MEDIUM/LOW]

## Suggested Fix
[How to address this issue]

## Found In
Code review of [version range or PR]
```

#### Labels for Code Review Issues:
- `code-review` - Primary label for all review findings
- Plus appropriate secondary label: `security`, `bug`, `enhancement`, `accessibility`, etc.

---

### Security Issues

**Security issues require immediate attention and specific handling.**

1. **Create issue** with `security` label
2. **Set priority label** to `priority:critical` or `priority:high`
3. **Do not include sensitive details** in public issues
4. **Fix immediately** before other work
5. **Close with verification** that the vulnerability is resolved

---

## Labels Convention

### Primary Labels (Required - Choose One)
| Label | Purpose |
|-------|---------|
| `feature:<name>` | Groups issues for a specific feature |
| `bug` | Something is broken |
| `enhancement` | Improvement to existing functionality |
| `feedback` | Direct user feedback/request |
| `code-review` | Finding from code review |
| `chore` | Maintenance, dependencies, config |
| `documentation` | Documentation updates |

### Secondary Labels (Optional - Add as Needed)
| Label | Purpose |
|-------|---------|
| `security` | Security-related issue |
| `ui` | UI/styling/CSS changes |
| `accessibility` | Accessibility improvements |
| `refactor` | Code restructuring |
| `performance` | Performance improvements |
| `status:in-progress` | Work has started |
| `status:blocked` | Blocked by another issue |

### Priority Labels (Required - Choose One)
| Label | Purpose |
|--------|---------|
| `priority:critical` | Security vulnerabilities, data loss, system down |
| `priority:high` | Broken functionality, major user impact |
| `priority:medium` | Important but not urgent |
| `priority:low` | Nice-to-have, minor improvements |

---

## GitHub CLI Commands Reference

```bash
# Create an issue with labels
gh issue create --title "Issue title" --label "bug,security,priority:high" --body "Issue body"

# Create an issue for user feedback
gh issue create --title "User feedback: improve modal UX" --label "feedback,ui,priority:medium" --body "## User Feedback
> 'The modal should close when clicking outside'

## Acceptance Criteria
- [ ] Modal closes on overlay click"

# Add label to existing issue
gh issue edit {number} --add-label "status:in-progress"

# Close an issue with comment
gh issue close {number} --comment "Fixed in commit abc123.

Files modified:
- src/handlers/example.rs
- templates/example.html"

# List open issues by label
gh issue list --label "bug" --state open

# List all open issues
gh issue list --state open

# View issue details
gh issue view {number}
```

---

## Enforcement

### Before Starting Any Work
1. Check if an issue exists for the work
2. If not, create one first
3. Never commit changes without a linked issue

### During Implementation
1. Reference issue number in commits when relevant
2. Update issue with progress if long-running
3. Check off acceptance criteria as completed

### After Completion
1. **Always close the issue** - never leave issues hanging
2. **Always include closing comment** with details
3. **Verify the change** works as expected before closing

### Review Checklist
- [ ] Issue exists before implementation started
- [ ] Appropriate labels applied
- [ ] Priority set in labels
- [ ] Issue closed with detailed comment
- [ ] Files modified listed in closing comment
