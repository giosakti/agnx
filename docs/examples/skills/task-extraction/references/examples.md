# Examples — Task Extraction Skill

## Example 1

Input:

> Can you extract tasks from this? We need to email Alex, book a room for Friday, and follow up on the invoice next week.

Expected output:

```json
{
  "tasks": [
    { "title": "Email Alex" },
    { "title": "Book a room for Friday" },
    { "title": "Follow up on the invoice", "due_date": "next week" }
  ]
}
```
