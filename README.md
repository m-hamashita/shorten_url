# URL Shortener

**Note**: This URL shortener application was created as a personal project for
practice and learning purposes.

This application allows users to shorten long URLs. It's built using Rust and uses
Docker for its runtime dependencies.

## Prerequisites

- Docker
- Rust

## Getting Started

1. Start Dependencies with Docker Compose:

   Before running the application, start the necessary dependencies using Docker
   Compose:

   ```bash
   docker compose up -d
   ```
   This will start the associated applications defined in compose.yaml.

1. Run the Rust Application:
   Navigate to the root directory of the project and run:
   ```bash
   cargo run
   ```
   This will start the URL shortener application.

1. Access the Application: Open your web browser and visit:
   ```
   http://localhost:3030
   ```

You'll be presented with a simple interface to shorten your URLs.

## Usage

1. Enter the URL you'd like to shorten.
1. Click on the "Shorten" button.
1. You'll be provided with a shortened link which you can use.
