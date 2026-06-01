# CookFetch Frontend Integration Guide: People, Households & Invites

This document covers the latest API features for managing **people profiles**, **household memberships**, and **QR-based invite system**.

**Base URL**: `http://localhost:4444` (or your deployment URL)

---

## 1. People Management (Settings)

People profiles allow users to store preferences and allergies for household members. This data is used to customize recipe suggestions and dietary guidance.

### 1.1 List All People

**Endpoint**: `GET /people/{userid}`

Get all people profiles for a specific user.

**Parameters**:

- `userid` (path, required): The user's unique ID from Firebase Auth

**Response** (200 OK):

```json
{
  "success": true,
  "people": [
    {
      "person_id": "person_abc123",
      "name": "Mom",
      "preference": ["vegetarian", "no fried food"],
      "allergy": ["peanut", "shellfish"]
    },
    {
      "person_id": "person_xyz789",
      "name": "Daughter",
      "preference": ["spicy", "extra rice"],
      "allergy": ["dairy"]
    }
  ],
  "count": 2
}
```

**Error Cases**:

- `500 Internal Server Error`: Failed to fetch people profiles

---

### 1.2 Create a Person Profile

**Endpoint**: `POST /people/{userid}`

Create a new person profile with dietary preferences and allergies.

**Parameters**:

- `userid` (path, required): The user's unique ID

**Request Body**:

```json
{
  "name": "Dad",
  "preference": ["no spicy", "extra vegetables"],
  "allergy": ["egg"]
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "person_id": "person_def456",
  "message": "Person profile created successfully"
}
```

**Validation Rules**:

- `name` is required and must not be empty
- `preference` and `allergy` are optional arrays of strings (empty arrays are allowed)

**Error Cases**:

- `400 Bad Request`: Missing or empty name
- `500 Internal Server Error`: Failed to create profile

---

### 1.3 Update a Person Profile

**Endpoint**: `PATCH /people/{userid}/{person_id}`

Update an existing person profile. All fields are optional for partial updates.

**Parameters**:

- `userid` (path, required): The user's unique ID
- `person_id` (path, required): The person profile ID to update

**Request Body** (all fields optional):

```json
{
  "name": "Grandma",
  "preference": ["soft food", "sweet"],
  "allergy": ["spicy", "citrus"]
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Person profile updated successfully"
}
```

**Notes**:

- Fields omitted from the request are not changed
- Providing empty arrays (`[]`) will clear that field
- At least one field must be provided to update

**Error Cases**:

- `400 Bad Request`: No fields to update
- `404 Not Found`: Person profile not found
- `500 Internal Server Error`: Failed to update profile

---

### 1.4 Delete a Person Profile

**Endpoint**: `DELETE /people/{userid}/{person_id}`

Delete a person profile permanently.

**Parameters**:

- `userid` (path, required): The user's unique ID
- `person_id` (path, required): The person profile ID to delete

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Person profile deleted successfully"
}
```

**Error Cases**:

- `404 Not Found`: Person profile not found
- `500 Internal Server Error`: Failed to delete profile

---

## 2. Household Management

Households enable multiple users to share schedules, grocery lists, and recipes. One user acts as the "owner" and approves member requests.

### 2.1 Create a Household

**Endpoint**: `POST /households`

Create a new household. The requesting user becomes the owner.

**Request Body**:

```json
{
  "owner_uid": "firebase_uid_here",
  "household_name": "Smith Family Kitchen"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "owner_uid": "firebase_uid_here",
  "household_name": "Smith Family Kitchen",
  "created_at": "2026-04-27T10:30:00Z"
}
```

**Notes**:

- The owner cannot be removed from the household
- The owner's user document is updated with the household_id
- Maximum 1 household per owner account

**Error Cases**:

- `400 Bad Request`: Missing required fields
- `500 Internal Server Error`: Failed to create household

---

### 2.2 Get My Household

**Endpoint**: `GET /households/my?member_uid={uid}`

Get the household that the current member belongs to.

**Query Parameters**:

- `member_uid` (required): The current user's Firebase UID

**Response** (200 OK):

```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "household_name": "Smith Family Kitchen",
  "owner_uid": "owner_id_here",
  "members": [
    {
      "uid": "owner_id",
      "role": "owner",
      "status": "approved",
      "joined_at": "2026-01-15T08:00:00Z"
    },
    {
      "uid": "mom_id",
      "role": "member",
      "status": "approved",
      "joined_at": "2026-02-20T14:30:00Z"
    }
  ],
  "member_count": 2
}
```

**Empty State**:

- If the user is not in a household, return `200 OK` with `{"household_id": null}`

**Error Cases**:

- `500 Internal Server Error`: Database error

---

### 2.3 Get Household Summary

**Endpoint**: `GET /households/{household_id}?member_uid={uid}`

Get full details of a household (requires membership).

**Parameters**:

- `household_id` (path, required): The household ID
- `member_uid` (query, required): The requesting member's UID

**Response** (200 OK):

```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "household_name": "Smith Family Kitchen",
  "owner_uid": "owner_id",
  "members": [
    {
      "uid": "owner_id",
      "role": "owner",
      "status": "approved",
      "joined_at": "2026-01-15T08:00:00Z"
    }
  ],
  "pending_requests": [
    {
      "request_id": "jr_xyz789",
      "member_uid": "dad_id",
      "status": "pending",
      "requested_at": "2026-04-27T12:00:00Z"
    }
  ]
}
```

**Error Cases**:

- `403 Forbidden`: Member is not in this household
- `404 Not Found`: Household not found

---

### 2.4 Request to Join a Household

**Endpoint**: `POST /households/{household_id}/join-requests`

Send a join request to a household. The owner must approve it.

**Parameters**:

- `household_id` (path, required): The target household

**Request Body**:

```json
{
  "member_uid": "user_requesting_to_join"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "request_id": "jr_abc123",
  "household_id": "hh_abc123xyz789",
  "member_uid": "user_requesting_to_join",
  "status": "pending",
  "requested_at": "2026-04-27T12:00:00Z"
}
```

**Error Cases**:

- `400 Bad Request`: User already in household
- `409 Conflict`: Pending request already exists
- `500 Internal Server Error`: Failed to create request

---

### 2.5 List Join Requests (Owner Only)

**Endpoint**: `GET /households/{household_id}/join-requests?actor_uid={uid}`

List all pending join requests for a household (owner only).

**Parameters**:

- `household_id` (path, required): The household
- `actor_uid` (query, required): The owner's UID

**Response** (200 OK):

```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "requests": [
    {
      "request_id": "jr_abc123",
      "member_uid": "dad_id",
      "status": "pending",
      "requested_at": "2026-04-27T12:00:00Z"
    },
    {
      "request_id": "jr_def456",
      "member_uid": "brother_id",
      "status": "pending",
      "requested_at": "2026-04-27T13:15:00Z"
    }
  ],
  "count": 2
}
```

**Error Cases**:

- `403 Forbidden`: Only owner can list requests
- `404 Not Found`: Household not found

---

### 2.6 Approve/Reject Join Request (Owner Only)

**Endpoint**: `PATCH /households/{household_id}/join-requests/{request_id}`

Approve or reject a pending join request.

**Parameters**:

- `household_id` (path, required): The household
- `request_id` (path, required): The join request ID
- `actor_uid` (query, required): The owner's UID

**Request Body**:

```json
{
  "status": "approved"
}
```

Accepted values: `"approved"` or `"rejected"`

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Join request approved",
  "member_uid": "dad_id"
}
```

**On Approval**:

- Member is automatically added with "member" role
- Member's user document is updated with household_id
- Join request is marked approved

**Error Cases**:

- `400 Bad Request`: Invalid status value
- `403 Forbidden`: Only owner can review requests
- `404 Not Found`: Request not found

---

### 2.7 Update Member Role (Owner Only)

**Endpoint**: `PATCH /households/{household_id}/members/{member_uid}`

Change a member's role (e.g., promote to "admin").

**Parameters**:

- `household_id` (path, required): The household
- `member_uid` (path, required): The member's UID
- `actor_uid` (query, required): The owner's UID

**Request Body**:

```json
{
  "role": "admin"
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Member role updated"
}
```

**Error Cases**:

- `400 Bad Request`: Cannot change owner role
- `403 Forbidden`: Only owner can update roles
- `404 Not Found`: Member not found

---

### 2.8 Remove Member (Owner Only)

**Endpoint**: `DELETE /households/{household_id}/members/{member_uid}`

Remove a member from the household.

**Parameters**:

- `household_id` (path, required): The household
- `member_uid` (path, required): The member to remove
- `actor_uid` (query, required): The owner's UID

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Member removed from household"
}
```

**Notes**:

- Owner cannot be removed
- Removed member's household_id is cleared from their user document

**Error Cases**:

- `400 Bad Request`: Cannot remove owner
- `403 Forbidden`: Only owner can remove members
- `404 Not Found`: Member not found

---

### 2.9 Leave Household (Member Self-Remove)

**Endpoint**: `POST /households/{household_id}/leave`

A member voluntarily leaves a household. Owner cannot use this endpoint.

**Request Body**:

```json
{
  "member_uid": "leaving_member_uid"
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Successfully left household"
}
```

**Error Cases**:

- `400 Bad Request`: Owner cannot leave (must delete household)
- `403 Forbidden`: Member not in household
- `404 Not Found`: Household not found

---

## 3. QR Code Invite System

The QR invite system enables quick household membership via scannable codes. Hosts generate codes, encode them as QR codes on the frontend, and members redeem codes to join instantly.

### 3.1 Generate Invite Code (Host Only)

**Endpoint**: `POST /households/{household_id}/generate-invite`

Generate a new QR invite code. Only the household owner can generate codes.

**Parameters**:

- `household_id` (path, required): The household

**Request Body**:

```json
{
  "actor_uid": "owner_uid_here"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "invite_code": "HH-ABC123",
  "household_id": "hh_abc123xyz789",
  "expires_at": "2026-04-28T10:45:30Z"
}
```

**Code Format**: `HH-XXXXXX` (6 uppercase alphanumeric characters)

**Valid Duration**: 24 hours from generation

**Usage on Frontend**:

1. Call this endpoint to get the code
2. Use a QR library (e.g., `qrcode.js`, `html5-qrcode`) to encode the code as QR
3. Display the QR code to members
4. Members scan the QR to retrieve the code
5. Members call the redeem endpoint with the code

**Error Cases**:

- `400 Bad Request`: Missing actor_uid
- `403 Forbidden`: Only owner can generate invites
- `404 Not Found`: Household not found

---

### 3.2 Redeem Invite Code (Any User)

**Endpoint**: `POST /households/{household_id}/redeem-invite`

Redeem a QR invite code to instantly join a household. No owner approval needed.

**Parameters**:

- `household_id` (path, required): The household

**Request Body**:

```json
{
  "invite_code": "HH-ABC123",
  "member_uid": "joining_member_uid"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "member_uid": "joining_member_uid",
  "message": "Successfully joined household!"
}
```

**On Success**:

- Member is automatically approved (no manual approval needed)
- Member's user document is updated with household_id
- Invite code is marked as redeemed
- Invite code cannot be used again

**Error Cases**:

- `400 Bad Request`: Invalid/missing fields or code expired
- `400 Bad Request`: Code already redeemed
- `404 Not Found`: Invite code not found
- `409 Conflict`: Member already in household

---

## 4. Household Schedules

Manage household-wide schedules (meals, events, etc.) accessible to all members.

### 4.1 List Household Schedules

**Endpoint**: `GET /households/{household_id}/schedules?member_uid={uid}`

List all schedules for a household.

**Parameters**:

- `household_id` (path, required): The household
- `member_uid` (query, required): Requesting member's UID

**Response** (200 OK):

```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "schedules": [
    {
      "schedule_id": "sch_abc123",
      "name": "Dinner Night",
      "date": "2026-04-28",
      "time": "18:00",
      "created_by": "owner_id",
      "created_at": "2026-04-27T10:00:00Z"
    }
  ],
  "count": 1
}
```

---

### 4.2 Create Household Schedule

**Endpoint**: `POST /households/{household_id}/schedules`

Create a new household schedule.

**Request Body**:

```json
{
  "name": "Lunch Party",
  "date": "2026-05-01",
  "time": "12:00",
  "creator_uid": "member_id"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "schedule_id": "sch_def456",
  "message": "Schedule created successfully"
}
```

---

### 4.3 Update Household Schedule

**Endpoint**: `PATCH /households/{household_id}/schedules/{schedule_id}`

Update a household schedule (creator only).

**Parameters**:

- `household_id` (path, required): The household
- `schedule_id` (path, required): The schedule ID
- `member_uid` (query, required): The creator's UID

**Request Body** (all fields optional):

```json
{
  "name": "Evening Dinner",
  "date": "2026-04-29",
  "time": "19:00"
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Schedule updated successfully"
}
```

---

### 4.4 Delete Household Schedule

**Endpoint**: `DELETE /households/{household_id}/schedules/{schedule_id}`

Delete a household schedule (creator only).

**Parameters**:

- `household_id` (path, required): The household
- `schedule_id` (path, required): The schedule ID
- `member_uid` (query, required): The creator's UID

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Schedule deleted successfully"
}
```

---

## 5. Household Grocery Lists

Manage shared household grocery lists.

### 5.1 List Grocery Lists

**Endpoint**: `GET /households/{household_id}/grocery?member_uid={uid}`

List all grocery lists for a household.

---

### 5.2 Create Grocery List

**Endpoint**: `POST /households/{household_id}/grocery`

Create a new grocery list.

**Request Body**:

```json
{
  "list_name": "Weekly Shopping",
  "creator_uid": "member_id"
}
```

---

### 5.3 Add Grocery Item

**Endpoint**: `POST /households/{household_id}/grocery/{list_id}/items`

Add an item to a grocery list.

**Request Body**:

```json
{
  "name": "Chicken",
  "quantity": "2 kg",
  "added_by": "member_id"
}
```

---

### 5.4 Update Grocery Item

**Endpoint**: `PATCH /households/{household_id}/grocery/{list_id}/items/{item_id}`

Update a grocery item.

**Request Body**:

```json
{
  "name": "Beef",
  "quantity": "1.5 kg",
  "status": "purchased"
}
```

---

### 5.5 Delete Grocery Item

**Endpoint**: `DELETE /households/{household_id}/grocery/{list_id}/items/{item_id}`

Delete a grocery item (creator only).

**Query Parameters**:

- `member_uid` (required): The creator's UID

---

### 5.6 Delete Grocery List

**Endpoint**: `DELETE /households/{household_id}/grocery/{list_id}`

Delete an entire grocery list (creator only).

**Query Parameters**:

- `member_uid` (required): The creator's UID

---

## 6. Chat with People Context

The chat endpoint now supports direct people context injection for recipe generation.

**Endpoint**: `POST /chat`

**Request Body**:

```json
{
  "prompt": "chicken, lemongrass, spicy",
  "promptlanguage": "en",
  "userid": "firebase_uid",
  "people_context": [
    {
      "name": "Mom",
      "preference": ["vegetarian", "no fried"],
      "allergy": ["peanut"]
    },
    {
      "name": "Sister",
      "preference": ["spicy"],
      "allergy": ["dairy"]
    }
  ]
}
```

**Response**:

```json
{
  "success": true,
  "recipes": [
    {
      "name": "Lemongrass Chicken Stir-Fry",
      "ingredients": ["chicken", "lemongrass", "..."],
      "steps": ["...", "..."],
      "suitable_for": ["Mom", "Sister"]
    }
  ]
}
```

**Notes**:

- `people_context` is optional
- If provided, recipes will consider the preferences and allergies
- Can be used instead of `selected_people_ids` for direct data injection
- Falls back to legacy endpoints if `people_context` not provided

---

## 7. Error Handling

### Common HTTP Status Codes

| Code | Meaning                              |
| ---- | ------------------------------------ |
| 200  | Request successful                   |
| 201  | Resource created successfully        |
| 400  | Bad request (missing/invalid fields) |
| 403  | Forbidden (insufficient permissions) |
| 404  | Resource not found                   |
| 409  | Conflict (e.g., duplicate record)    |
| 500  | Server error                         |

### Standard Error Response

```json
{
  "error": "Description of what went wrong",
  "details": "Optional additional context"
}
```

---

## 8. Implementation Tips

### QR Code Generation (Frontend)

```javascript
// Example using qrcode.js
import QRCode from "qrcode";

async function generateQRCode(inviteCode) {
  const canvas = document.getElementById("qr-canvas");
  await QRCode.toCanvas(canvas, inviteCode, {
    width: 300,
    margin: 2,
    color: {
      dark: "#000000",
      light: "#ffffff",
    },
  });
}

// Usage
const response = await fetch("/households/hh_abc123xyz789/generate-invite", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({ actor_uid: "owner_id" }),
});

const { invite_code } = await response.json();
await generateQRCode(invite_code);
```

### QR Code Scanning (Frontend)

```javascript
// Example using html5-qrcode
import Html5QrcodeScanner from "html5-qrcode/esm/html5-qrcode-scanner.js";

const scanner = new Html5QrcodeScanner("reader", {
  fps: 10,
  qrbox: { width: 250, height: 250 },
});

scanner.render(async (decodedText) => {
  // decodedText contains the scanned invite code (e.g., "HH-ABC123")
  const response = await fetch("/households/hh_abc123xyz789/redeem-invite", {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      invite_code: decodedText,
      member_uid: "current_user_id",
    }),
  });

  const result = await response.json();
  if (result.success) {
    console.log("Successfully joined household!");
    // Redirect to household dashboard
  }
});
```

### Managing User Household

```javascript
// Get the current user's household
const response = await fetch("/households/my?member_uid=current_user_id");
const household = await response.json();

if (household.success) {
  console.log("Household ID:", household.household_id);
  console.log("Members:", household.members);
} else {
  console.log("User is not in a household");
}
```

### Managing People Profiles

```javascript
// Fetch all people for the user
const response = await fetch("/people/user_id");
const { people } = await response.json();

// Create a new person
await fetch("/people/user_id", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    name: "Grandpa",
    preference: ["soft food"],
    allergy: ["spicy", "citrus"],
  }),
});

// Update a person
await fetch("/people/user_id/person_abc123", {
  method: "PATCH",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    preference: ["mild", "sweet"],
  }),
});

// Delete a person
await fetch("/people/user_id/person_abc123", {
  method: "DELETE",
});
```

---

## 9. Permissions Summary

| Feature            | Owner  | Member | Non-Member |
| ------------------ | ------ | ------ | ---------- |
| Generate Invites   | ✓      | ✗      | ✗          |
| Redeem Invites     | ✓      | ✓      | ✓          |
| Approve Requests   | ✓      | ✗      | ✗          |
| Remove Members     | ✓      | ✗      | ✗          |
| Update Roles       | ✓      | ✗      | ✗          |
| Leave Household    | ✓ (NO) | ✓      | ✗          |
| Create Schedules   | ✓      | ✓      | ✗          |
| Edit Own Schedules | ✓      | ✓      | ✗          |
| View Household     | ✓      | ✓      | ✗          |
| Manage Grocery     | ✓      | ✓      | ✗          |

---

## 10. Deprecated Endpoints

The following legacy endpoints are deprecated but still functional:

- `PATCH /add/schedules/{uuid}`
- `PATCH /change/schedules/{uuid}`
- `DELETE /delete/schedules/{uuid}/{schedule_key}`

**⚠️ Warning**: These endpoints will be removed in a future version. Use the new household endpoints instead.

---

## Need Help?

For API issues, check:

1. The error response message
2. The permissions table above
3. The example code in Section 8
4. Ensure all required query/path parameters are provided
5. Verify user is in the correct household

Last updated: April 27, 2026
