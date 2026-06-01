# Frontend Migration Guide: Collection-Based Firestore API

Last updated: 2026-04-19

This guide explains how to migrate frontend code from the old map-based backend model to the new collection-based model.

Use this as the source of truth for frontend Copilot prompts and API integration.

## 1. Migration Summary

Previous backend model:
- user document stored large map fields:
- `grocery`, `savedrecipes`, `schedules`

New backend model:
- root user doc stays small (profile/hardware/basic fields)
- collection data is stored in subcollections and fetched separately

Current Firestore layout:
- `users/{uid}`
- `users/{uid}/grocery/{groceryId}`
- `users/{uid}/grocery/{groceryId}/items/{itemId}`
- `users/{uid}/savedrecipes/{recipeId}`
- `users/{uid}/schedules/{scheduleId}`
- `users/{uid}/people/{personId}` (exists in Firestore, API routes for people may be added separately)

## 2. Important Frontend Contract Rules

1. Never send Firestore REST shape (`fields`, `stringValue`, `mapValue`) from frontend.
2. Always send clean JSON payloads.
3. Frontend should consume IDs returned by backend (`uid`, `list_id`, etc.), never parse Firestore document path strings.
4. Keep date and time formats consistent:
- date: `YYYY-MM-DD`
- time: `HH-MM`

## 3. Live API Routes (from main router)

Base URL (default local):
- `http://localhost:4444`

Core:
- `GET /user/{uid}`
- `GET /checktime`

Schedule + recipe management:
- `PATCH /add/schedules/{uuid}`
- `PATCH /add/recipes/{uuid}`
- `PATCH /change/schedules/{uuid}`
- `DELETE /delete/schedules/{uuid}/{schedule_key}`
- `DELETE /delete/recipes/{uuid}/{recipe_key}`

Notify:
- `PATCH /notify/on/{uuid}/{schedule_id}`
- `PATCH /notify/off/{uuid}/{schedule_id}`
- `GET /notify/get_notify/{uuid}`

Grocery:
- `GET /grocery/{userid}`
- `POST /grocery/{userid}`
- `GET /grocery/{userid}/{list_id}`
- `DELETE /grocery/{userid}/{list_id}`
- `PATCH /grocery/{userid}/{list_id}/items`
- `PATCH /grocery/{userid}/{list_id}/items/{item_id}`
- `DELETE /grocery/{userid}/{list_id}/items/{item_id}`
- `POST /grocery/{userid}/{list_id}/items/{item_id}/to-schedule`

People (settings):
- `GET /people/{userid}`
- `POST /people/{userid}`
- `PATCH /people/{userid}/{person_id}`
- `DELETE /people/{userid}/{person_id}`

Households (shared mode):
- `POST /households`
- `GET /households/{household_id}?member_uid={uid}`
- `POST /households/{household_id}/join-requests`
- `GET /households/{household_id}/join-requests?actor_uid={owner_uid}`
- `PATCH /households/{household_id}/join-requests/{request_id}`
- `PATCH /households/{household_id}/members/{member_uid}`
- `DELETE /households/{household_id}/members/{member_uid}`
- `GET /households/{household_id}/schedules?member_uid={uid}`
- `POST /households/{household_id}/schedules`
- `GET /households/{household_id}/grocery?member_uid={uid}`
- `POST /households/{household_id}/grocery`

## 4. Endpoint Contracts You Need for Frontend Refactor

### 4.1 Add schedule

Endpoint:
- `PATCH /add/schedules/{uuid}`

Request body:
```json
{
  "name": "Buy milk",
  "time": "09-30",
  "date": "2026-04-21",
  "expiry": "2026-04-25",
  "notify": true,
  "category": "grocery",
  "quantity": "2 bottles",
  "location": "Kitchen"
}
```

Notes:
- `category` optional
- `location` optional
- Frontend should add a `location` input to the add-schedule form and include it in the request when present.
- If the field is empty, omit `location` or send `null` depending on the request serializer.

### 4.2 Change schedules (current compatibility endpoint)

Endpoint:
- `PATCH /change/schedules/{uuid}`

Current backend still accepts a Firestore-shaped compatibility payload here.

Request body currently expected:
```json
{
  "fields": {
    "schedules": {
      "mapValue": {
        "fields": {
          "schedule_abc": {
            "mapValue": {
              "fields": {
                "location": { "stringValue": "Fridge" },
                "time": { "stringValue": "11-30" }
              }
            }
          }
        }
      }
    }
  }
}
```

Frontend recommendation:
- isolate this compatibility shape in one adapter function
- do not spread this payload shape across UI code

### 4.3 Delete schedule

Endpoint:
- `DELETE /delete/schedules/{uuid}/{schedule_key}`

### 4.4 Add saved recipe

Endpoint:
- `PATCH /add/recipes/{uuid}`

Request body:
```json
{
  "name": "Tom Yum Soup",
  "ingredients": ["water", "lemongrass", "shrimp"],
  "steps": ["boil water", "add ingredients"],
  "time": "30 min",
  "image_url": "https://...",
  "quantity": "2 bowls"
}
```

### 4.5 Delete saved recipe

Endpoint:
- `DELETE /delete/recipes/{uuid}/{recipe_key}`

### 4.6 Grocery list CRUD

Create list:
- `POST /grocery/{userid}`

Request body:
```json
{
  "list_name": "Weekly Shopping"
}
```

Add item:
- `PATCH /grocery/{userid}/{list_id}/items`

Request body:
```json
{
  "name": "Eggs",
  "quantity": "12",
  "unit": "pcs"
}
```

Update item:
- `PATCH /grocery/{userid}/{list_id}/items/{item_id}`

Request body (partial allowed):
```json
{
  "checked": true,
  "quantity": "18",
  "unit": "pcs"
}
```

### 4.7 Convert grocery item to schedule

Endpoint:
- `POST /grocery/{userid}/{list_id}/items/{item_id}/to-schedule`

Request body:
```json
{
  "date": "2026-04-21",
  "time": "10-15",
  "expiry": "2026-04-28",
  "category": "grocery",
  "location": "Pantry"
}
```

### 4.8 User aggregate fetch

Endpoint:
- `GET /user/{uid}`

Response shape:
```json
{
  "email": "user@example.com",
  "username": "demo",
  "savedrecipes": [
    {
      "uid": "recipe_x",
      "name": "Tom Yum",
      "ingredients": ["..."],
      "steps": ["..."],
      "time": "...",
      "image_url": "...",
      "quantity": "..."
    }
  ],
  "grocery": [
    {
      "list_id": "grocery_x",
      "list_name": "Weekly Shopping",
      "items": [
        {
          "uid": "item_x",
          "name": "Eggs",
          "quantity": "12",
          "unit": "pcs",
          "checked": false
        }
      ]
    }
  ],
  "schedules": [
    {
      "uid": "schedule_x",
      "name": "Buy milk",
      "time": "09-30",
      "date": "2026-04-21",
      "expiry": "2026-04-25",
      "category": "grocery",
      "quantity": "2 bottles",
      "location": "Kitchen"
    }
  ],
  "people": [
    {
      "person_id": "person_x",
      "name": "Aunt_tie",
      "preference": ["like/dislike milk", "extra : infos"],
      "allergy": ["peanut", "water"]
    }
  ],
  "hardware": {
    "devices": ["..."],
    "requests": ["..."]
  }
}
```

### 4.9 People CRUD (settings)

List people:
- `GET /people/{userid}`

Create person:
- `POST /people/{userid}`

Request body:
```json
{
  "name": "Aunt_tie",
  "preference": ["like/dislike milk", "extra : infos"],
  "allergy": ["peanut", "water"]
}
```

Update person (partial allowed):
- `PATCH /people/{userid}/{person_id}`

Request body:
```json
{
  "name": "Auntie",
  "preference": ["no spicy"],
  "allergy": ["shrimp"]
}
```

Delete person:
- `DELETE /people/{userid}/{person_id}`

### 4.10 Assistant with selected people context

Endpoint:
- `POST /chat`

New request fields (optional):
- `userid`: user document id for loading people profiles
- `selected_people_ids`: array of person ids to inject into assistant system instructions

Request body excerpt:
```json
{
  "prompt": "chicken, lemongrass",
  "promptlanguage": "en",
  "userid": "GOahOpwSRMhMDeYTLx8GGWDJRQ13",
  "selected_people_ids": ["person_abc123"]
}
```

Behavior:
- if `selected_people_ids` is provided, assistant prompt includes those people preferences and allergies
- if omitted/empty, assistant behavior is unchanged

### 4.11 Household sharing with host approval

Create household (host):
- `POST /households`

Request body:
```json
{
  "owner_uid": "mom_uid_demo",
  "name": "My Family"
}
```

Join household request (dad/son):
- `POST /households/{household_id}/join-requests`

Request body:
```json
{
  "requester_uid": "dad_uid_demo",
  "requested_role": "member"
}
```

Host reviews join request:
- `PATCH /households/{household_id}/join-requests/{request_id}`

Request body:
```json
{
  "actor_uid": "mom_uid_demo",
  "action": "approve"
}
```

Shared schedules (approved members only):
- `GET /households/{household_id}/schedules?member_uid={uid}`
- `POST /households/{household_id}/schedules`

Shared schedule create body:
```json
{
  "member_uid": "dad_uid_demo",
  "name": "Buy Milk",
  "time": "09-30",
  "date": "2026-04-28",
  "expiry": "2026-05-02",
  "notify": true,
  "category": "grocery",
  "quantity": "2 bottles",
  "location": "Kitchen"
}
```

Shared grocery lists (approved members only):
- `GET /households/{household_id}/grocery?member_uid={uid}`
- `POST /households/{household_id}/grocery`

Shared grocery list create body:
```json
{
  "member_uid": "dad_uid_demo",
  "list_name": "Weekly Shared"
}
```

Approval rule:
- pending members are blocked from shared schedule/grocery writes until host approves.

## 5. TypeScript Types for Frontend

```ts
export type Schedule = {
  uid: string;
  name: string;
  time: string;      // HH-MM
  date: string;      // YYYY-MM-DD
  expiry: string;    // YYYY-MM-DD
  category?: string | null;
  quantity?: string | null;
  location?: string | null;
  notify?: boolean;
};

export type AddScheduleRequest = {
  name: string;
  time: string;
  date: string;
  expiry: string;
  notify: boolean;
  category?: string;
  quantity: string;
  location?: string;
};

export type AddRecipeRequest = {
  name: string;
  ingredients: string[];
  steps: string[];
  time?: string;
  image_url?: string;
  quantity?: string;
};

export type GroceryList = {
  list_id: string;
  list_name: string;
  items: GroceryItem[];
};

export type GroceryItem = {
  uid: string;
  name: string;
  quantity: string;
  unit: string;
  checked: boolean;
};

export type CreateGroceryListRequest = {
  list_name: string;
};

export type AddGroceryItemRequest = {
  name: string;
  quantity: string;
  unit: string;
};

export type UpdateGroceryItemRequest = {
  checked?: boolean;
  quantity?: string;
  unit?: string;
};

export type PersonProfile = {
  person_id: string;
  name: string;
  preference: string[];
  allergy: string[];
};

export type CreatePersonRequest = {
  name: string;
  preference: string[];
  allergy: string[];
};

export type UpdatePersonRequest = {
  name?: string;
  preference?: string[];
  allergy?: string[];
};

export type ChatPeopleContext = {
  userid?: string;
  selected_people_ids?: string[];
};
```

## 6. Copilot-Friendly Prompt Template for Frontend Changes

Use this prompt in frontend Copilot chat when refactoring:

```text
Refactor this frontend module to the new collection-based backend contract.
Rules:
1) Never send Firestore REST payload shapes (fields/stringValue/mapValue).
2) Use clean JSON request/response types from docs/schedule-location-frontend.md.
3) For schedule update endpoint only, call a small adapter to build compatibility payload.
4) Do not parse Firestore document paths; use uid/list_id/item_id from API responses.
5) Keep existing UI behavior unchanged.
```

## 7. Suggested Frontend Refactor Order

1. Replace all old assumptions that user root contains grocery/schedules/savedrecipes maps.
2. Centralize API calls in one module (`apiClient.ts` or equivalent).
3. Add strict request/response types from this doc.
4. Keep one compatibility adapter for `PATCH /change/schedules/{uuid}`.
5. Remove any remaining direct Firestore payload logic from components.

## 8. Known Compatibility Note

Current backend still accepts Firestore-shaped payload for schedule update route (`PATCH /change/schedules/{uuid}`).

Plan to simplify later to a clean JSON patch contract. Keep your adapter isolated so this backend change becomes a one-file frontend update.
