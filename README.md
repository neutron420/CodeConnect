<div align="center">

# CodeConnect

<br/>

<div>
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Rust">
  <img src="https://img.shields.io/badge/TypeScript-3178C6?style=for-the-badge&logo=typescript&logoColor=white" alt="TypeScript">
  <img src="https://img.shields.io/badge/Actix_Web-000000?style=for-the-badge&logo=rust&logoColor=white" alt="Actix Web">
  <img src="https://img.shields.io/badge/React-61DAFB?style=for-the-badge&logo=react&logoColor=black" alt="React">
  <img src="https://img.shields.io/badge/SQLx-000000?style=for-the-badge&logo=postgresql&logoColor=white" alt="SQLx">
  <img src="https://img.shields.io/badge/Docker-2496ED?style=for-the-badge&logo=docker&logoColor=white" alt="Docker">
  <img src="https://img.shields.io/badge/Vercel-000000?style=for-the-badge&logo=vercel&logoColor=white" alt="Vercel">
</div>

<br/>

**A high-performance online code compiler and execution engine built with Rust backend and modern web technologies. Compile and run code in multiple programming languages with secure sandboxed execution, real-time output, and blazing-fast performance.**

<p>
  <a href="#about-the-project">About</a> •
  <a href="#key-features">Features</a> •
  <a href="#getting-started">Getting Started</a> •
  <a href="#contributing">Contributing</a> •
  <a href="#license">License</a>
</p>

[**Live Demo**](https://compiler-nine-phi.vercel.app) · [**Documentation**](https://ritesh-docs.gitbook.io/ritesh-docs-docs/deployment) · [**Report a Bug**](https://github.com/neutron420/CodeConnect/issues) · [**Request a Feature**](https://github.com/neutron420/CodeConnect/issues)

</div>

## About The Project

CodeConnect is a robust online compiler platform that allows users to write, compile, and execute code in multiple programming languages directly from their browser. Built with a high-performance Rust backend using Actix-Web framework and a modern TypeScript/React frontend, it provides a secure, sandboxed environment for code execution with real-time output streaming. Perfect for learning, teaching, quick prototyping, or running code snippets without local setup.

### Built With

This project combines the performance of Rust with modern web technologies for an optimal development experience.

* **Backend Framework:** [Rust](https://www.rust-lang.org/) with [Actix-Web](https://actix.rs/)
* **Frontend:** [TypeScript](https://www.typescriptlang.org/), [React](https://react.dev/)
* **Database:** [SQLx](https://github.com/launchbadge/sqlx) (PostgreSQL)
* **Containerization:** [Docker](https://www.docker.com/), [Docker Compose](https://docs.docker.com/compose/)
* **Deployment:** [Vercel](https://vercel.com/) (Frontend)
* **CI/CD:** [GitHub Actions](https://github.com/features/actions)
* **Git Hooks:** [Husky](https://typicode.github.io/husky/)

## Key Features

* **Multi-Language Support:** Compile and execute code in multiple programming languages
* **Blazing Fast Performance:** Rust-powered backend for lightning-fast compilation and execution
* **Secure Sandboxing:** Isolated execution environment for safe code running
* **Real-Time Output:** Live streaming of compilation and execution results
* **Code Persistence:** Save and manage your code snippets with database integration
* **Modern UI/UX:** Clean, intuitive interface built with React and TypeScript
* **RESTful API:** Well-documented API for programmatic access
* **Docker Support:** Containerized deployment for consistent environments
* **Database Integration:** SQLx for type-safe database operations
* **CI/CD Pipeline:** Automated testing and deployment with GitHub Actions
* **Error Handling:** Comprehensive error messages for compilation and runtime errors
* **Resource Management:** Memory and execution time limits for security

## Getting Started

To get a local copy up and running for development, follow these simple steps.

### Prerequisites

You will need the following installed on your system:

* **Rust** (latest stable version)
  ```sh
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
  ```

* **Node.js** (version 18 or higher)
  ```sh
  # Download from https://nodejs.org/
  ```

* **Docker & Docker Compose** (optional, for containerized deployment)
  ```sh
  # Download from https://www.docker.com/
  ```

* **PostgreSQL** (if running without Docker)
  ```sh
  # Download from https://www.postgresql.org/
  ```

### Installation

1.  **Clone the repository:**
    ```sh
    git clone https://github.com/neutron420/CodeConnect.git
    cd CodeConnect
    ```

2.  **Set up environment variables:**
    Create a `.env` file in the root directory:
    ```env
    DATABASE_URL=postgresql://user:password@localhost:5432/codeconnect
    RUST_LOG=info
    SERVER_HOST=127.0.0.1
    SERVER_PORT=8080
    ```

3.  **Set up the database:**
    ```sh
    # Run database migrations
    sqlx database create
    sqlx migrate run
    ```

4.  **Install frontend dependencies:**
    ```sh
    cd web
    npm install
    ```

5.  **Install Git hooks:**
    ```sh
    npm run prepare
    ```

### Running the Application

#### Option A: Using Docker Compose (Recommended)

```sh
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f
```

The application will be available at:
- Backend API: http://localhost:8080
- Frontend: http://localhost:3000

#### Option B: Running Locally

**Terminal 1 - Backend:**
```sh
# Run the Rust backend
cargo run
```

**Terminal 2 - Frontend:**
```sh
# Run the web frontend
cd web
npm run dev
```

## Project Structure

```
codeconnect/
├── src/                   # Rust backend source code
│   ├── main.rs           # Application entry point
│   ├── routes/           # API route handlers
│   ├── models/           # Data models
│   ├── handlers/         # Request handlers
│   └── compiler/         # Code compilation logic
├── web/                   # Frontend application
│   ├── src/              # React/TypeScript source
│   ├── public/           # Static assets
│   └── package.json      # Frontend dependencies
├── db/                    # Database migrations and scripts
├── .sqlx/                 # SQLx metadata
├── .husky/                # Git hooks
├── .github/workflows/     # CI/CD workflows
├── Dockerfile             # Docker configuration
├── docker-compose.yml     # Docker Compose setup
├── Cargo.toml            # Rust dependencies
└── package.json          # Root package configuration
```

## API Endpoints

### Compilation API

```sh
# Compile and execute code
POST /api/compile
Content-Type: application/json

{
  "language": "rust",
  "code": "fn main() { println!(\"Hello, World!\"); }",
  "input": ""
}
```

### Code Management

```sh
# Save code snippet
POST /api/snippets

# Get user snippets
GET /api/snippets

# Get specific snippet
GET /api/snippets/:id

# Update snippet
PUT /api/snippets/:id

# Delete snippet
DELETE /api/snippets/:id
```

For complete API documentation, visit the [Documentation](https://ritesh-docs.gitbook.io/ritesh-docs-docs/deployment).

## Supported Languages

CodeConnect currently supports:

* Rust
* Python
* JavaScript/Node.js
* C/C++
* Java
* Go
* And more...

## Development

### Running Tests

```sh
# Run backend tests
cargo test

# Run frontend tests
cd web
npm test
```

### Building for Production

```sh
# Build Rust backend
cargo build --release

# Build frontend
cd web
npm run build
```

### Database Migrations

```sh
# Create a new migration
sqlx migrate add <migration_name>

# Run migrations
sqlx migrate run

# Revert last migration
sqlx migrate revert
```

## Deployment

The project is deployed on:

* **Frontend:** Vercel - [compiler-nine-phi.vercel.app](https://compiler-nine-phi.vercel.app)
* **Backend:** Self-hosted / Cloud provider

### Deploy Your Own

1. **Frontend to Vercel:**
   ```sh
   cd web
   vercel deploy
   ```

2. **Backend with Docker:**
   ```sh
   docker build -t codeconnect-backend .
   docker run -p 8080:8080 codeconnect-backend
   ```

## Documentation

Comprehensive documentation is available at:
[https://ritesh-docs.gitbook.io/ritesh-docs-docs/deployment](https://ritesh-docs.gitbook.io/ritesh-docs-docs/deployment)

## Security Considerations

* **Sandboxed Execution:** All code runs in isolated containers
* **Resource Limits:** CPU and memory constraints prevent abuse
* **Timeout Protection:** Execution time limits for all programs
* **Input Validation:** Strict validation of user inputs
* **Rate Limiting:** API rate limits to prevent spam

## Technologies Deep Dive

### Why Rust for Backend?

* **Performance:** Near-native speed for compilation tasks
* **Safety:** Memory safety without garbage collection
* **Concurrency:** Fearless concurrency with ownership system
* **Actix-Web:** One of the fastest web frameworks available

### Frontend Architecture

* **React:** Component-based UI development
* **TypeScript:** Type safety and better developer experience
* **Modern Tooling:** Vite for fast development and building

## Contributing

Contributions are what make the open-source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".

1.  Fork the Project
2.  Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3.  Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4.  Push to the Branch (`git push origin feature/AmazingFeature`)
5.  Open a Pull Request

## License

Distributed under the MIT License. See `LICENSE` for more information.

## Contact

Project Link: [https://github.com/neutron420/CodeConnect](https://github.com/neutron420/CodeConnect)

Live Demo: [https://compiler-nine-phi.vercel.app](https://compiler-nine-phi.vercel.app)

Documentation: [https://ritesh-docs.gitbook.io/ritesh-docs-docs/deployment](https://ritesh-docs.gitbook.io/ritesh-docs-docs/deployment)

## Acknowledgments

* [Rust Programming Language](https://www.rust-lang.org/)
* [Actix Web Framework](https://actix.rs/)
* [SQLx](https://github.com/launchbadge/sqlx)
* [React Documentation](https://react.dev/)
* [Vercel Platform](https://vercel.com/)
