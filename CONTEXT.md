# Work Notes

Work Notes captures source notes and derives action items that can be followed across their lifecycle.

## Language

**Action item**:
A task derived from a source note whose current state is suggested, open, or done.
_Avoid_: Calendar event

**Captured date**:
The date the source note containing an action item was captured. It is the action item's stable origin on the Calendar.
_Avoid_: Created date, task-created date

**Due date**:
The optional date by which an action item is expected to be completed.
_Avoid_: Scheduled date

**Completion date**:
The date an action item entered the done state, when that history is available.
_Avoid_: Archive date

**Calendar occurrence**:
A dated representation of an action item at its captured, due, or completion date. Multiple occurrences belong to the same action item.
_Avoid_: Duplicate task

**Action-item lifecycle**:
The connected sequence of captured, optional due, and completion moments for one action item. Its dated moments are distinct from its current suggested, open, or done state.
_Avoid_: Ticket states

**Completed note**:
A source note whose work was finished and intentionally removed from the active inbox.
_Avoid_: Archived note, deleted note

**Ticket**:
A Jira issue shown in the Jira view. Tickets live in Jira; the app stores only the keys of tickets created from notes.
_Avoid_: Action item, task

**Lookup**:
A read-only Jira request the agent makes, such as a search or fetching one ticket. Lookups run without asking.
_Avoid_: Tool call (for user-facing text)

**View change**:
Something the agent does to the app's own Jira view: putting a ticket search in the list or opening a ticket. View changes run without asking because Jira is not touched.
_Avoid_: Proposed change (that term is for changes to Jira)

**Proposed change**:
A Jira change the agent asked for, stored in the conversation until the user approves or rejects it. Only an approved proposed change reaches Jira.
_Avoid_: Pending action, suggested action

**Archived note**:
A source note intentionally set aside without claiming that its work was completed. It remains recoverable as a soft-deleted record.
_Avoid_: Completed note, done note
