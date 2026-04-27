# Backend Note: Household Lookup Returns 404 for Empty State

Observed behavior:
- `GET /households/my?member_uid=GOahOpwSRMhMDeYTLx8GGWDJRQ13`
- Response body: `{"household_id": null}`
- The frontend treats this as an error during startup because the endpoint returns `404` instead of a normal empty response.

Suggested backend behavior:
- If "no household" is a valid state, return `200 OK` with `{"household_id": null}` or an empty object.
- If `404 Not Found` is intentional, the frontend should continue treating it as a normal empty state and not log it as a startup failure.

Impact:
- This affects app startup because the frontend loads the current user's household immediately.
- The issue is not related to People CRUD.
