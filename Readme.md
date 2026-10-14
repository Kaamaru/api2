# API2

A Rust backend API for household planning, schedule management, notifications, grocery tracking, recipe search, and AI-assisted features.

Built with **Rust + Axum**, backed by **Firebase/Firestore**, and integrated with **Gemini AI**, image hosting, and device/hardware workflows.

## Features

**Household & users**
- User, people/profile, and household management
- Schedule creation and updates with event notifications
- Grocery lists tied to schedules

**Recipes**
- Recipe search and detail lookup
- AI-generated recipe recommendations
- Save recipes to user lists

**AI & media**
- Gemini-powered chat
- Image classification and batch detection
- Image upload via ImgBB

**Hardware & integrations**
- Device connection flow with request/approval workflow
- Hardware records and status endpoints
- LINE integration
- Email notifications and reminders

## Tech Stack

| Area | Tools |
|------|-------|
| Language / runtime | Rust, Tokio |
| Web framework | Axum, Tower HTTP (CORS) |
| Data | Firebase / Firestore |
| AI | Gemini API, tract (ONNX) |
| HTTP & serialization | Reqwest, Serde |
| Media | ImgBB |

## Project Structure

```text
api2/
├── src/
│   ├── main.rs
│   ├── firebase.rs
│   ├── parse.rs
│   ├── structures/        # models, structs, web structs
│   ├── tasks/
│   │   ├── basic.rs
│   │   ├── batch_detect.rs
│   │   ├── classify_mock.rs
│   │   ├── getimage.rs
│   │   ├── sendmail.rs
│   │   ├── timecheck.rs
│   │   ├── firestore/
│   │   ├── gemini/
│   │   ├── hardware/
│   │   ├── line/
│   │   └── recipe_search/
│   └── tools/
│       ├── imagebb.rs
│       └── keycrypt.rs
├── Cargo.toml
└── docs/
```

## Getting Started

### Prerequisites

- Rust and Cargo ([install](https://rustup.rs))
- A Firebase project
- A Gemini API key

### Installation

```bash
git clone https://github.com/Kaamaru/api2.git
cd api2
cargo build
```

### Configuration

Create a `.env` file in the project root:

```env
GEMINI_API_KEY=your_gemini_api_key
GEMINI_URL=your_gemini_base_url

FIREBASE_API_KEY=your_firebase_api_key
FIREBASE_AUTH_DOMAIN=your_project.firebaseapp.com
FIREBASE_DATABASE_URL=https://your-project-default-rtdb.firebaseio.com
FIREBASE_DATABASE=your_database_name
FIREBASE_PROJECT_ID=your_project_id
FIREBASE_STORAGE_BUCKET=your-project.appspot.com
FIREBASE_MESSAGING_SENDER_ID=your_sender_id
FIREBASE_APP_ID=your_app_id
FIREBASE_MEASUREMENT_ID=your_measurement_id
```

> Never commit `.env` or real credentials. Make sure `.env` is listed in `.gitignore`.

### Run

```bash
cargo run
```

The server listens on port `4444` by default. Override it with `PORT`:

```bash
PORT=8080 cargo run
```

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `/` | Base / health route |
| `/chat` | Gemini chat |
| `/classify` | Image classification |
| `/batch-detect` | Batch detection |
| `/user/{uid}` | Fetch user data |
| `/grocery` | Grocery list operations |
| `/people` | People/profile management |
| `/households` | Household management |
| `/hardware` | Device and hardware records |
| `/line` | LINE integration |
| `/search/recipes` | Recipe search and generation |
