# Household Firestore Structure Refactoring

## Overview

The household system has been refactored to use a simplified Firestore structure with a flat members array instead of subcollection memberships and join requests.

## Old vs New Structure

### Old Structure
```
/households/{household_id}
  /members/{member_uid}  <- Subcollection with status, role
  /join_requests/{request_id}  <- Subcollection for pending requests
Fields: owner_uid, created_at
```

### New Structure
```
/households/{household_id}
Fields:
  - host: string (uid of owner)
  - name: string (household name)
  - members: string[] (approved member uids only, host NOT included)
  - invite_code: string | null (active QR invite code)
  - invite_expires_at: string | null (ISO timestamp when code expires)
```

**Key differences:**
- Host is stored as a string field, separate from members array
- Members array contains approved member UIDs only (host is NOT in array)
- Single flat document - no subcollections
- Invite codes are stored on root document (one-time use, then cleared)

---

## Updated Endpoints

### 1. POST /households (Create)

**Request:**
```json
{
  "owner_uid": "firebase_uid",
  "name": "Smith Family Kitchen"
}
```

**Changes:**
- Creates household with `host` field (not `owner_uid`)
- Initializes empty `members` array `[]`
- Sets `invite_code` and `invite_expires_at` to null
- Stores `household_id` on `users/{owner_uid}.household_id`

**Response:**
```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "name": "Smith Family Kitchen",
  "owner_uid": "firebase_uid"
}
```

---

### 2. GET /households/my?member_uid={uid} (Get User's Household)

**Changes:**
- Reads `users/{uid}.household_id` to find user's household
- Fetches `/households/{household_id}` document
- Returns `host`, `name`, `members` array, and boolean `is_host`

**Response:**
```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "name": "Smith Family Kitchen",
  "host": "owner_uid",
  "members": ["member_uid_1", "member_uid_2"],
  "is_host": false
}
```

---

### 3. POST /households/{household_id}/generate-invite (New - Host Only)

**Request:**
```json
{
  "actor_uid": "owner_uid"
}
```

**Changes:**
- Verifies `actor_uid == host` (not subcollection lookup)
- Generates code format: `HH-XXXXXX` (6 random uppercase alphanumeric)
- Sets `invite_code` and `invite_expires_at` (24 hours) on root document
- One-time use: clears fields when redeemed

**Response:**
```json
{
  "success": true,
  "invite_code": "HH-ABC123",
  "household_id": "hh_abc123xyz789",
  "expires_at": "2026-04-28T10:45:30Z"
}
```

---

### 4. POST /households/redeem-invite (New - Any User)

**Endpoint changed:** No longer includes household_id in path

**Request:**
```json
{
  "invite_code": "HH-ABC123",
  "member_uid": "joining_member_uid"
}
```

**Changes:**
- Searches all households for matching `invite_code`
- Validates not expired
- Adds `member_uid` to `members` array
- Clears `invite_code` and `invite_expires_at` (one-time use)
- Stores `household_id` on `users/{member_uid}.household_id`
- No owner approval needed - instant join

**Response:**
```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "name": "Smith Family Kitchen",
  "member_uid": "joining_member_uid",
  "message": "Successfully joined household!"
}
```

---

### 5. DELETE /households/{household_id}/members/{member_uid}?actor_uid={uid} (Host Removes Member)

**Changes:**
- Verifies `actor_uid == host`
- Removes `member_uid` from `members` array
- Cannot remove the host
- Clears `users/{member_uid}.household_id`

**Response:**
```json
{
  "success": true,
  "message": "Member removed"
}
```

---

### 6. POST /households/{household_id}/leave (Member Self-Remove)

**Request:**
```json
{
  "member_uid": "leaving_member_uid"
}
```

**Changes:**
- Member removes themselves from `members` array
- Host cannot leave (returns 400)
- Clears `users/{member_uid}.household_id`

**Response:**
```json
{
  "success": true,
  "message": "Successfully left household"
}
```

---

### 7. GET /households/{household_id}/schedules?member_uid={uid} (List Schedules)

**Changes:**
- Verifies member_uid is in `members` array OR is the `host`
- Single permission check instead of subcollection lookup

---

### 8. POST /households/{household_id}/schedules (Create Schedule)

**Changes:**
- Member uid comes from request body (not query param for create)
- Verifies membership using new array structure

---

## Removed Endpoints

These endpoints no longer exist:
- `GET /households/{household_id}` - Use `GET /households/my` instead
- `POST /households/{household_id}/join-requests` - Use QR invite redeem
- `GET /households/{household_id}/join-requests?actor_uid={uid}` - Removed (no join requests)
- `PATCH /households/{household_id}/join-requests/{request_id}` - Removed (no join requests)
- `PATCH /households/{household_id}/members/{member_uid}` - Role updates removed

---

## Permission Model

All endpoints verify: `member_uid == host` OR `member_uid in members[]`

```rust
// Permission check pattern
async fn is_member_or_host(
    client: &Client,
    firebase_config: &FirebaseConfig,
    household_id: &str,
    member_uid: &str,
) -> Result<bool, String> {
    let household_doc = fetch_household(household_id).await?;
    let host = get_doc_string_field(&household_doc, "host");
    let members = get_doc_array_field(&household_doc, "members");
    
    Ok(member_uid == host || members.contains(&member_uid))
}
```

---

## Grocery Lists & Schedules

Both now use the simplified permission check:
- GET lists: Verify membership, fetch subcollection
- POST/PATCH/DELETE: Verify membership, operate on item
- Query parameters: `member_uid` required for all operations

**Example:**
```
GET /households/{household_id}/schedules?member_uid=user_id
```

---

## Database Migrations

### For existing households:

```
For each old household {household_id}:
1. Read old members subcollection
2. Extract approved members into array: ["uid1", "uid2", ...]
3. Read owner_uid field
4. Create new root document with:
   - host: owner_uid
   - name: household_name
   - members: [approved_uids] (excluding owner)
   - invite_code: null
   - invite_expires_at: null
5. Delete old members subcollection
6. Delete old join_requests subcollection
```

---

## Testing Checklist

- [ ] Create household → host stored correctly
- [ ] Get my household → returns host, members, is_host
- [ ] Generate invite → code format HH-XXXXXX, expiry set
- [ ] Redeem invite → member added to array, code cleared
- [ ] Remove member → host only, removes from array
- [ ] Leave household → member removes self, host cannot leave
- [ ] Schedule CRUD → permission check works
- [ ] Grocery CRUD → permission check works
- [ ] Expired invite → rejects with "code has expired"
- [ ] Already member → rejects with "member already in household"
- [ ] Non-member access → returns 403 Forbidden

---

## Breaking Changes

1. **GET /households/{household_id}** removed
   - Use `GET /households/my?member_uid={uid}` instead
   
2. **Join request flow** completely removed
   - Old: POST join-request → owner approves
   - New: POST redeem-invite → instant approval
   
3. **invite_code endpoint URL changed**
   - Old: `POST /households/{household_id}/redeem-invite`
   - New: `POST /households/redeem-invite` (no household_id needed)

4. **Member operations now host-only**
   - No member role updates
   - Only host can remove members

---

## Implementation Files

- **src/tasks/firestore/household.rs** - Refactored with new structure
  - create_household()
  - get_my_household()
  - generate_household_invite()
  - redeem_household_invite()
  - remove_member()
  - leave_household()
  - is_member_or_host() helper

- **src/tasks/firestore/household_extended.rs** - Schedules & grocery
  - All endpoints updated to use new permission model
  - is_member_or_host() helper for access control

- **src/main.rs** - Router configuration
  - Removed old join-request routes
  - Added new invite routes
  - Simplified household router

---

## Frontend Integration Notes

### For existing features:
- Update all `GET /households/{household_id}` calls to `GET /households/my?member_uid={uid}`
- Update all member removal to use only host endpoint
- Remove all join-request approval UI

### For new QR invite flow:
1. Host calls `POST /households/{household_id}/generate-invite`
2. Frontend encodes response `invite_code` as QR
3. Member scans QR, sends code to `POST /households/redeem-invite`
4. Member automatically added to household

---

## Error Handling

All endpoints return consistent error responses:

```json
{
  "error": "Description of what went wrong",
  "details": "Optional technical details"
}
```

Status codes:
- 200: Success
- 201: Created
- 400: Bad request (validation, invalid status)
- 403: Forbidden (not host, not member, insufficient permissions)
- 404: Not found (household, schedule, item)
- 409: Conflict (already member, duplicate, etc)
- 500: Server error

---

**Last Updated**: April 27, 2026
