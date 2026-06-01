# Frontend Integration Guide - CookFetch API v2

Complete API reference for integrating CookFetch household, scheduling, and grocery management features into your frontend.

## Table of Contents

1. [Person Management](#1-person-management)
2. [Household Management](#2-household-management)
3. [QR Invite System](#3-qr-invite-system)
4. [Household Schedules](#4-household-schedules)
5. [Grocery Lists](#5-grocery-lists)
6. [Recipe Search & Integration](#6-recipe-search--integration)
7. [Chat Endpoints](#7-chat-endpoints)
8. [Error Handling](#8-error-handling)
9. [JavaScript Examples](#9-javascript-examples)
10. [Permission Matrix](#10-permission-matrix)

---

## 1. Person Management

Person profiles store basic information about individual household members (name, photo, preferences, etc.).

### 1.1 Create Person Profile

**Endpoint**: `POST /people/{userid}`

Create a new person profile for a user.

**Parameters**:

- `userid` (path, required): The Firebase user ID

**Request Body**:

```json
{
  "name": "John Smith",
  "email": "john@example.com",
  "photo_url": "https://example.com/photo.jpg",
  "preferences": "Vegetarian, no peanuts"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "person_id": "person_abc123xyz",
  "userid": "firebase_uid",
  "name": "John Smith",
  "created_at": "2026-04-27T10:30:00Z"
}
```

**Error Cases**:

- `400 Bad Request`: Missing required fields
- `500 Internal Server Error`: Failed to create profile

---

### 1.2 Get Person Profiles

**Endpoint**: `GET /people/{userid}`

Retrieve all person profiles for a user.

**Parameters**:

- `userid` (path, required): The Firebase user ID

**Response** (200 OK):

```json
{
  "success": true,
  "profiles": [
    {
      "person_id": "person_abc123xyz",
      "name": "John Smith",
      "email": "john@example.com",
      "photo_url": "https://example.com/photo.jpg",
      "preferences": "Vegetarian, no peanuts",
      "created_at": "2026-04-27T10:30:00Z"
    }
  ],
  "count": 1
}
```

**Error Cases**:

- `404 Not Found`: User not found
- `500 Internal Server Error`: Failed to retrieve profiles

---

### 1.3 Update Person Profile

**Endpoint**: `PATCH /people/{userid}/{person_id}`

Update an existing person profile.

**Parameters**:

- `userid` (path, required): The Firebase user ID
- `person_id` (path, required): The person profile ID

**Request Body** (all fields optional):

```json
{
  "name": "John Smith Updated",
  "email": "newemail@example.com",
  "photo_url": "https://example.com/newphoto.jpg",
  "preferences": "Vegetarian, no peanuts, dairy-free"
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "person_id": "person_abc123xyz",
  "message": "Person profile updated successfully"
}
```

**Error Cases**:

- `404 Not Found`: Person profile not found
- `500 Internal Server Error`: Failed to update profile

---

### 1.4 Delete Person Profile

**Endpoint**: `DELETE /people/{userid}/{person_id}`

Delete a person profile.

**Parameters**:

- `userid` (path, required): The Firebase user ID
- `person_id` (path, required): The person profile ID

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

Households enable multiple users to share schedules, grocery lists, and recipes. One user acts as the "host" and can invite others via QR code.

### 2.1 Create Household

**Endpoint**: `POST /households`

Create a new household. The requesting user becomes the host.

**Request Body**:

```json
{
  "owner_uid": "firebase_uid",
  "name": "Smith Family Kitchen"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "name": "Smith Family Kitchen",
  "owner_uid": "firebase_uid"
}
```

**Storage**:

- Creates `/households/{household_id}` with:
  - `host`: owner_uid (string, the host's Firebase UID)
  - `name`: household name (string)
  - `members`: [] (empty array initially)
  - `invite_code`: null
  - `invite_expires_at`: null
- Updates `users/{owner_uid}.household_id` with the new household_id

**Error Cases**:

- `400 Bad Request`: Missing owner_uid or name
- `500 Internal Server Error`: Failed to create household

---

### 2.2 Get My Household

**Endpoint**: `GET /households/my?member_uid={uid}`

Retrieve the household for the current user (if member of one).

**Parameters**:

- `member_uid` (query, required): The current user's Firebase UID

**Response** (200 OK):

```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "name": "Smith Family Kitchen",
  "host": "host_firebase_uid",
  "members": ["member_uid_1", "member_uid_2"],
  "is_host": false,
  "created_at": "2026-04-27T10:30:00Z"
}
```

**Fields Explained**:

- `host`: The Firebase UID of the household owner (host cannot be removed)
- `members`: Array of member UIDs (does NOT include host)
- `is_host`: Boolean indicating if requesting user is the host

**Empty State**:

- If the user is not part of any household, return `200 OK` with `{"household_id": null}`

**Error Cases**:

- `500 Internal Server Error`: Failed to retrieve household

---

### 2.3 Remove a Member (Host Only)

**Endpoint**: `DELETE /households/{household_id}/members/{member_uid}?actor_uid={uid}`

Remove a member from the household. Only the host can do this.

**Parameters**:

- `household_id` (path, required): The household
- `member_uid` (path, required): The member to remove
- `actor_uid` (query, required): The host's UID (for permission check)

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Member removed from household"
}
```

**Effects**:

- Removes `member_uid` from the `members` array
- Clears `users/{member_uid}.household_id`
- Host cannot be removed (returns 400)

**Error Cases**:

- `400 Bad Request`: Attempted to remove the host
- `403 Forbidden`: Only the host can remove members
- `404 Not Found`: Member or household not found
- `500 Internal Server Error`: Failed to remove member

---

### 2.4 Leave Household (Member Self-Remove)

**Endpoint**: `POST /households/{household_id}/leave`

Leave a household (member removes themselves).

**Parameters**:

- `household_id` (path, required): The household to leave

**Request Body**:

```json
{
  "member_uid": "firebase_uid"
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Successfully left household"
}
```

**Effects**:

- Removes requesting user from `members` array
- Clears `users/{member_uid}.household_id`
- Host cannot leave (returns 400)

**Error Cases**:

- `400 Bad Request`: Host cannot leave household
- `403 Forbidden`: User is not a member
- `404 Not Found`: Household not found
- `500 Internal Server Error`: Failed to leave household

---

## 3. QR Invite System

The QR invite system enables quick household membership via scannable codes. Hosts generate codes, encode them as QR codes on the frontend, and members redeem codes to join instantly with no approval needed.

**Flow**:

1. Host calls `/households/{household_id}/generate-invite` → receives code "HH-XXXXXX"
2. Frontend encodes code as QR code image
3. Member scans QR → gets code
4. Member calls `/households/redeem-invite` → joins instantly
5. No approval step needed (unlike old join-request system)

### 3.1 Generate Invite Code (Host Only)

**Endpoint**: `POST /households/{household_id}/generate-invite`

Generate a new QR invite code. Only the host can do this.

**Parameters**:

- `household_id` (path, required): The household

**Request Body**:

```json
{
  "actor_uid": "host_firebase_uid"
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "invite_code": "HH-ABC123",
  "household_id": "hh_abc123xyz789",
  "expires_at": "2026-04-28T10:45:30Z"
}
```

**Code Format**:

- Prefix: `HH-` (for "Household")
- 6 random uppercase alphanumeric characters: `A-Z`, `0-9`
- Example: `HH-K7P2MQ`, `HH-ABC123`
- **One-time use**: Code is cleared from the household document after first successful redemption
- **Expires**: 24 hours after generation

**Storage**:

- Sets `invite_code` field on household document
- Sets `invite_expires_at` to current time + 24 hours (ISO 8601 format)
- Previous code (if any) is overwritten

**Error Cases**:

- `403 Forbidden`: Only the host can generate invites
- `404 Not Found`: Household not found
- `500 Internal Server Error`: Failed to generate code

---

### 3.2 Redeem Invite Code (Any User)

**Endpoint**: `POST /households/redeem-invite`

Redeem a QR invite code to instantly join a household. No owner approval needed.

**Request Body**:

```json
{
  "invite_code": "HH-ABC123",
  "member_uid": "user_joining_firebase_uid"
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "household_id": "hh_abc123xyz789",
  "name": "Smith Family Kitchen",
  "member_uid": "user_joining_firebase_uid",
  "message": "Successfully joined household!"
}
```

**How It Works**:

1. Searches all households for matching `invite_code`
2. Validates code has not expired
3. Validates member is not already in household
4. Adds `member_uid` to the `members` array
5. Clears `invite_code` and `invite_expires_at` (one-time use)
6. Updates `users/{member_uid}.household_id` with household_id
7. Returns success with household details

**Effects**:

- User is immediately added to household (no approval step)
- Code is marked as consumed (fields set to null)
- User can now access all household schedules and grocery lists

**Error Cases**:

- `400 Bad Request`: Code has expired or is invalid
- `409 Conflict`: User already member of this household
- `404 Not Found`: No household found with that code
- `500 Internal Server Error`: Failed to redeem code

---

## 4. Household Schedules

Schedules are shared calendars for household members. Members can add events like meal prep, grocery shopping, or family gatherings.

### 4.1 List Household Schedules

**Endpoint**: `GET /households/{household_id}/schedules?member_uid={uid}`

List all schedules for a household.

**Parameters**:

- `household_id` (path, required): The household
- `member_uid` (query, required): The requesting member's UID

**Response** (200 OK):

```json
{
  "success": true,
  "schedules": [
    {
      "schedule_id": "sch_abc123",
      "household_id": "hh_abc123xyz789",
      "title": "Weekly Meal Prep",
      "description": "Sunday meal prep session",
      "date": "2026-04-27",
      "time": "14:00",
      "duration_minutes": 120,
      "location": "Kitchen",
      "created_by": "owner_uid",
      "created_at": "2026-04-25T10:30:00Z"
    }
  ],
  "count": 1
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `403 Forbidden`: User is not a member of this household
- `404 Not Found`: Household not found
- `500 Internal Server Error`: Failed to retrieve schedules

---

### 4.2 Add Household Schedule

**Endpoint**: `POST /households/{household_id}/schedules`

Create a new schedule event.

**Parameters**:

- `household_id` (path, required): The household

**Request Body**:

```json
{
  "member_uid": "firebase_uid",
  "title": "Weekly Meal Prep",
  "description": "Sunday meal prep session",
  "date": "2026-04-27",
  "time": "14:00",
  "duration_minutes": 120,
  "location": "Kitchen"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "schedule_id": "sch_abc123",
  "household_id": "hh_abc123xyz789",
  "title": "Weekly Meal Prep",
  "created_at": "2026-04-25T10:30:00Z"
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `400 Bad Request`: Missing required fields
- `403 Forbidden`: User is not a member
- `404 Not Found`: Household not found
- `500 Internal Server Error`: Failed to create schedule

---

### 4.3 Update Household Schedule

**Endpoint**: `PATCH /households/{household_id}/schedules/{schedule_id}`

Update an existing schedule.

**Parameters**:

- `household_id` (path, required): The household
- `schedule_id` (path, required): The schedule to update

**Request Body** (all fields optional):

```json
{
  "member_uid": "firebase_uid",
  "title": "Updated Title",
  "description": "Updated description",
  "date": "2026-04-28",
  "time": "15:00",
  "duration_minutes": 90,
  "location": "Updated Location"
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "schedule_id": "sch_abc123",
  "message": "Schedule updated successfully"
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `403 Forbidden`: User is not a member
- `404 Not Found`: Schedule or household not found
- `500 Internal Server Error`: Failed to update schedule

---

### 4.4 Delete Household Schedule

**Endpoint**: `DELETE /households/{household_id}/schedules/{schedule_id}?member_uid={uid}`

Delete a schedule.

**Parameters**:

- `household_id` (path, required): The household
- `schedule_id` (path, required): The schedule to delete
- `member_uid` (query, required): The requesting member's UID

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Schedule deleted successfully"
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `403 Forbidden`: User is not a member
- `404 Not Found`: Schedule or household not found
- `500 Internal Server Error`: Failed to delete schedule

---

## 5. Grocery Lists

Shared grocery lists help household members coordinate shopping and meal planning.

### 5.1 List Household Grocery Lists

**Endpoint**: `GET /households/{household_id}/grocery?member_uid={uid}`

List all grocery lists for a household.

**Parameters**:

- `household_id` (path, required): The household
- `member_uid` (query, required): The requesting member's UID

**Response** (200 OK):

```json
{
  "success": true,
  "lists": [
    {
      "list_id": "groc_abc123",
      "household_id": "hh_abc123xyz789",
      "name": "Weekly Shopping",
      "description": "Groceries for week of April 27",
      "items": [
        {
          "item_id": "item_1",
          "name": "Milk",
          "quantity": "2",
          "unit": "liters",
          "purchased": false,
          "added_by": "member_uid_1",
          "added_at": "2026-04-25T10:30:00Z"
        }
      ],
      "created_by": "member_uid_1",
      "created_at": "2026-04-25T10:30:00Z"
    }
  ],
  "count": 1
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `403 Forbidden`: User is not a member
- `404 Not Found`: Household not found
- `500 Internal Server Error`: Failed to retrieve lists

---

### 5.2 Create Household Grocery List

**Endpoint**: `POST /households/{household_id}/grocery`

Create a new grocery list.

**Parameters**:

- `household_id` (path, required): The household

**Request Body**:

```json
{
  "member_uid": "firebase_uid",
  "name": "Weekly Shopping",
  "description": "Groceries for week of April 27"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "list_id": "groc_abc123",
  "household_id": "hh_abc123xyz789",
  "name": "Weekly Shopping",
  "created_at": "2026-04-25T10:30:00Z"
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `400 Bad Request`: Missing required fields
- `403 Forbidden`: User is not a member
- `404 Not Found`: Household not found
- `500 Internal Server Error`: Failed to create list

---

### 5.3 Delete Grocery List

**Endpoint**: `DELETE /households/{household_id}/grocery/{list_id}?member_uid={uid}`

Delete an entire grocery list.

**Parameters**:

- `household_id` (path, required): The household
- `list_id` (path, required): The list to delete
- `member_uid` (query, required): The requesting member's UID

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Grocery list deleted successfully"
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `403 Forbidden`: User is not a member
- `404 Not Found`: List or household not found
- `500 Internal Server Error`: Failed to delete list

---

### 5.4 Add Grocery Item

**Endpoint**: `POST /households/{household_id}/grocery/{list_id}/items`

Add an item to a grocery list.

**Parameters**:

- `household_id` (path, required): The household
- `list_id` (path, required): The grocery list

**Request Body**:

```json
{
  "member_uid": "firebase_uid",
  "name": "Milk",
  "quantity": "2",
  "unit": "liters"
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "item_id": "item_1",
  "list_id": "groc_abc123",
  "name": "Milk",
  "quantity": "2",
  "unit": "liters",
  "purchased": false,
  "created_at": "2026-04-25T10:30:00Z"
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `400 Bad Request`: Missing required fields
- `403 Forbidden`: User is not a member
- `404 Not Found`: List or household not found
- `500 Internal Server Error`: Failed to add item

---

### 5.5 Update Grocery Item

**Endpoint**: `PATCH /households/{household_id}/grocery/{list_id}/items/{item_id}`

Update a grocery item (e.g., mark as purchased).

**Parameters**:

- `household_id` (path, required): The household
- `list_id` (path, required): The grocery list
- `item_id` (path, required): The item to update

**Request Body** (all fields optional):

```json
{
  "member_uid": "firebase_uid",
  "name": "Milk",
  "quantity": "2",
  "unit": "liters",
  "purchased": true
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "item_id": "item_1",
  "message": "Item updated successfully"
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `403 Forbidden`: User is not a member
- `404 Not Found`: Item, list, or household not found
- `500 Internal Server Error`: Failed to update item

---

### 5.6 Delete Grocery Item

**Endpoint**: `DELETE /households/{household_id}/grocery/{list_id}/items/{item_id}?member_uid={uid}`

Delete a grocery item.

**Parameters**:

- `household_id` (path, required): The household
- `list_id` (path, required): The grocery list
- `item_id` (path, required): The item to delete
- `member_uid` (query, required): The requesting member's UID

**Response** (200 OK):

```json
{
  "success": true,
  "message": "Item deleted successfully"
}
```

**Permission**:

- Requesting user must be the host OR in the members array

**Error Cases**:

- `403 Forbidden`: User is not a member
- `404 Not Found`: Item, list, or household not found
- `500 Internal Server Error`: Failed to delete item

---

## 6. Recipe Search & Integration

Search for recipes and integrate them into grocery lists.

### 6.1 Search Recipes

**Endpoint**: `GET /search/recipes?query={query}&limit={limit}`

Search for recipes by keyword.

**Parameters**:

- `query` (query, required): Search term (e.g., "pasta", "chicken")
- `limit` (query, optional): Maximum results to return (default: 10)

**Response** (200 OK):

```json
{
  "success": true,
  "recipes": [
    {
      "recipe_id": "rec_abc123",
      "title": "Simple Pasta Carbonara",
      "description": "Classic Italian pasta dish",
      "ingredients": [
        {"item": "pasta", "quantity": "400", "unit": "g"},
        {"item": "eggs", "quantity": "4", "unit": ""},
        {"item": "bacon", "quantity": "200", "unit": "g"}
      ],
      "instructions": "1. Cook pasta...",
      "prep_time": 15,
      "cook_time": 20,
      "servings": 4,
      "source": "spoonacular"
    }
  ],
  "count": 1
}
```

**Error Cases**:

- `400 Bad Request`: Missing query parameter
- `500 Internal Server Error`: Search failed

---

### 6.2 Get Recipe Details

**Endpoint**: `GET /search/recipes/{recipe_id}`

Get detailed information about a specific recipe.

**Parameters**:

- `recipe_id` (path, required): The recipe ID

**Response** (200 OK):

```json
{
  "success": true,
  "recipe_id": "rec_abc123",
  "title": "Simple Pasta Carbonara",
  "description": "Classic Italian pasta dish",
  "ingredients": [
    {"item": "pasta", "quantity": "400", "unit": "g"},
    {"item": "eggs", "quantity": "4", "unit": ""},
    {"item": "bacon", "quantity": "200", "unit": "g"}
  ],
  "instructions": "1. Cook pasta...",
  "prep_time": 15,
  "cook_time": 20,
  "servings": 4,
  "nutrition": {
    "calories": 450,
    "protein": 25,
    "carbs": 45,
    "fat": 18
  },
  "source": "spoonacular"
}
```

**Error Cases**:

- `404 Not Found`: Recipe not found
- `500 Internal Server Error`: Failed to retrieve recipe

---

### 6.3 Add Recipe to Grocery List

**Endpoint**: `POST /search/recipes/{recipe_id}/add-to-list`

Add all ingredients from a recipe to a grocery list.

**Parameters**:

- `recipe_id` (path, required): The recipe ID

**Request Body**:

```json
{
  "member_uid": "firebase_uid",
  "household_id": "hh_abc123xyz789",
  "list_id": "groc_abc123",
  "servings": 4
}
```

**Response** (201 Created):

```json
{
  "success": true,
  "list_id": "groc_abc123",
  "items_added": 12,
  "message": "Recipe ingredients added to grocery list"
}
```

**How It Works**:

1. Fetches recipe details
2. Scales ingredients based on serving size
3. Adds each ingredient to specified grocery list
4. Returns count of items added

**Error Cases**:

- `400 Bad Request`: Missing required fields
- `403 Forbidden`: User is not a member of household
- `404 Not Found`: Recipe, list, or household not found
- `500 Internal Server Error`: Failed to add items

---

## 7. Chat Endpoints

AI-powered chat for recipe suggestions, meal planning, and nutrition advice.

### 7.1 Send Chat Message

**Endpoint**: `POST /chat`

Send a message to the AI chat (with household context if applicable).

**Request Body**:

```json
{
  "user_uid": "firebase_uid",
  "household_id": "hh_abc123xyz789",
  "message": "What recipes can I make with chicken and rice?"
}
```

**Response** (200 OK):

```json
{
  "success": true,
  "response": "Here are some delicious chicken and rice recipes: ...",
  "message_id": "msg_abc123",
  "timestamp": "2026-04-27T10:45:30Z"
}
```

**Context**:

- If `household_id` is provided, AI has access to household members and preferences
- Message history is maintained per user

**Error Cases**:

- `400 Bad Request`: Missing message text
- `403 Forbidden`: Household member check failed
- `500 Internal Server Error`: AI service error

---

## 8. Error Handling

All API endpoints return consistent error responses.

### Standard Error Response

```json
{
  "error": "Brief error description",
  "details": "Optional technical details",
  "code": "ERROR_CODE"
}
```

### HTTP Status Codes

| Status | Meaning | Common Cause |
|--------|---------|--------------|
| 200 | OK | Request successful |
| 201 | Created | Resource created successfully |
| 400 | Bad Request | Invalid input, missing fields, validation error |
| 403 | Forbidden | Insufficient permissions, not host/member |
| 404 | Not Found | Resource does not exist |
| 409 | Conflict | Resource already exists, duplicate request |
| 500 | Server Error | Firestore error, service failure |

### Common Errors

**403 Forbidden - Not a Member**:

```json
{
  "error": "Forbidden",
  "details": "User is not a member of this household"
}
```

**400 Bad Request - Expired Code**:

```json
{
  "error": "Invalid invite code",
  "details": "Code has expired or is invalid"
}
```

**409 Conflict - Already Member**:

```json
{
  "error": "Conflict",
  "details": "User already member of this household"
}
```

---

## 9. JavaScript Examples

### 9.1 Create Household

```javascript
async function createHousehold() {
  const response = await fetch('/households', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      owner_uid: currentUser.uid,
      name: 'Smith Family Kitchen'
    })
  });

  const data = await response.json();
  if (data.success) {
    const householdId = data.household_id;
    console.log('Household created:', householdId);
  }
}
```

### 9.2 Generate & Display QR Code

```javascript
async function generateQRInvite(householdId) {
  const response = await fetch(
    `/households/${householdId}/generate-invite`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ actor_uid: currentUser.uid })
    }
  );

  const data = await response.json();
  if (data.success) {
    const inviteCode = data.invite_code;
    const expiresAt = data.expires_at;

    // Generate QR code from inviteCode
    // Use QR library: qrcode.js or similar
    const qrCanvas = await generateQRCode(inviteCode);
    document.getElementById('qr-container').appendChild(qrCanvas);

    // Show expiry time
    console.log('Invite expires at:', expiresAt);
  }
}
```

### 9.3 Scan & Redeem QR Code

```javascript
async function redeemInviteCode(scannedCode) {
  const response = await fetch('/households/redeem-invite', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      invite_code: scannedCode,
      member_uid: currentUser.uid
    })
  });

  const data = await response.json();
  if (data.success) {
    console.log('Successfully joined:', data.household_id);
    console.log('Household name:', data.name);
    // Update UI to show household
  } else {
    console.error('Failed to redeem code:', data.error);
  }
}
```

### 9.4 Add to Grocery List

```javascript
async function addGroceryItem(householdId, listId, itemName) {
  const response = await fetch(
    `/households/${householdId}/grocery/${listId}/items`,
    {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({
        member_uid: currentUser.uid,
        name: itemName,
        quantity: '1',
        unit: 'item'
      })
    }
  );

  const data = await response.json();
  if (data.success) {
    console.log('Item added:', data.item_id);
  }
}
```

### 9.5 List Household Schedules

```javascript
async function listSchedules(householdId) {
  const response = await fetch(
    `/households/${householdId}/schedules?member_uid=${currentUser.uid}`,
    { method: 'GET' }
  );

  const data = await response.json();
  if (data.success) {
    console.log('Schedules:', data.schedules);
    data.schedules.forEach((schedule) => {
      console.log(`${schedule.title} on ${schedule.date} at ${schedule.time}`);
    });
  }
}
```

### 9.6 Chat with AI

```javascript
async function chatWithAI(message, householdId) {
  const response = await fetch('/chat', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      user_uid: currentUser.uid,
      household_id: householdId,
      message: message
    })
  });

  const data = await response.json();
  if (data.success) {
    console.log('AI Response:', data.response);
    return data.response;
  }
}
```

---

## 10. Permission Matrix

This matrix shows which operations are allowed for which user roles.

| Operation | Host | Member | Non-Member |
|-----------|------|--------|------------|
| Get household | ✓ | ✓ | ✗ |
| Invite members | ✓ | ✗ | ✗ |
| Redeem invite | N/A | ✓ | ✓ |
| Remove member | ✓ | ✗ | ✗ |
| Leave household | ✓* | ✓ | ✗ |
| View schedules | ✓ | ✓ | ✗ |
| Add schedule | ✓ | ✓ | ✗ |
| Update schedule | ✓ | ✓ | ✗ |
| Delete schedule | ✓ | ✓ | ✗ |
| View grocery lists | ✓ | ✓ | ✗ |
| Create grocery list | ✓ | ✓ | ✗ |
| Add grocery item | ✓ | ✓ | ✗ |
| Update grocery item | ✓ | ✓ | ✗ |
| Delete grocery item | ✓ | ✓ | ✗ |

**Notes**:

- `*Host cannot leave household - only members can leave or be removed by host`
- Non-members cannot access any household data
- All operations require valid Firebase authentication
- Household ID must be known or obtained via `/households/my`

---

**Last Updated**: April 27, 2026
**API Version**: v2 (Refactored QR Invite System)
