# This is a comment. Recipes start with their name followed by a colon.

# Recipe for running the development server (useful for Axum projects)
default:
    @cargo run

# Recipe for building the in dev and facilitating restart
start:
    @bacon run

# Recipe for building the production release artifact
build-prod:
    @cargo build --release

# Build docker
build-docker:
    @cargo docker build -t fuelfinder_server -f ./rust_fuelfinder_docker/Dockerfile .

# Recipe for running all tests
test:
    @cargo test

# Recipe for building in development
build-dev:
    @cargo build

# Recipe for cleanup (removing compiled artifacts)
clean:
    @cargo clean

# Recipe for a multi-step task (e.g., format, check, then test)
ci:
    just fmt
    just check
    just test

# Recipe with a variable argument (using a shell command)
hello MESSAGE='default message' :
    @echo "Hello, you sent: {{MESSAGE}}"

# Recipe for starting docker database
db_start:
	@docker volume create rustfuelfinder_data && docker-compose -f 'rust_fuelfinder_docker/postgres.compose.yaml' up --build

# Recipe for stopping docker database
db_stop:
	@docker-compose -f 'rust_fuelfinder_docker/postgres.compose.yaml' down

reset_db:
    @sqlx database drop && sqlx database create && sqlx migrate run

# sqlx migrate add <description>