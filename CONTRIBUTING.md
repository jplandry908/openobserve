# Contributing to OpenObserve

## Setting up development environment

### Prerequisite

OpenObserve uses Rust & embdeds sled db (For server) & VueJS (For Web UI)

You must have following installed:

1. Git
2. Rust & Cargo 1.70+
3. nodejs v14+ and npm v6+

Alternatively you can use pre-configured devcontainer in VS code to get up and running quickly.

## Building from source code

### Lets clone the repo and get started

```shell
git clone https://github.com/openobserve/openobserve
cd openobserve
```

### Now let's build the UI

```shell
cd web
npm install
npm run build
cd ..
```

Output will be stored in web/dist folder. web/dist will be embedded in openobserve binary when openobserve application is built.

It is important that you build the web app every time you make any changes to javascript code as the built code is then embedded in openobserve application.

### Time to build the rust application now

Make sure you are in openobserve directory & not in openobserve/web, and have
Rust toolchain installed:

Install `protoc`:

> Make sure the protoc is >= 3.15

```
# Linux
wget https://github.com/protocolbuffers/protobuf/releases/download/v21.12/protoc-21.12-linux-x86_64.zip
unzip protoc-21.12-linux-x86_64.zip -d protoc
sudo cp protoc/bin/protoc /usr/local/bin/
sudo cp -r protoc/include/google /usr/local/include/

# Windows
curl -L -o protoc-21.12-win64.zip https://github.com/protocolbuffers/protobuf/releases/download/v21.12/protoc-21.12-win64.zip
unzip protoc-21.12-win64.zip -d protoc
cp protoc/bin/protoc.exe C:/windows/system32/protoc.exe
cp -r protoc/include/google C:/windows/system32/

# macOS
brew install protobuf

# Set environment variable for protobuf include path (macOS)
export PROTOC_INCLUDE=/opt/homebrew/include
# Add this to your shell profile (~/.zshrc, ~/.bashrc, etc.) to make it permanent
You could also set it in your .env file
```

Build the binary:

```shell
cargo build --release
```

## Developing

Once you have the source code cloned you can start development.

There are 2 areas of development.

1. UI
1. Server

### Server

```shell
ZO_ROOT_USER_EMAIL="root@example.com" ZO_ROOT_USER_PASSWORD="Complexpass#123" cargo run
```

This will start the openobserve API server on port 5080

environment variables ZO_ROOT_USER_EMAIL and ZO_ROOT_USER_PASSWORD can be used first time to configure default admin user when openobserve is started.

### UI

If not already present create .env file in web directory & set VITE_OPENOBSERVE_ENDPOINT=http://localhost:5080

```shell
cd web
npm install
npm run dev
```

This will start UI server on port 8081, to change port specify it in vite.config.ts

In order for you to effectively use the UI you would want to have the openobserve API server running in a separate window that will accept requests from the UI.

## Swagger

The server also exposes a Swagger API endpoint which you can see by visiting the `/swagger/index.html` path. It uses [utoipa](https://github.com/juhaku/utoipa) to mark API endpoints with comment annotations & to generate the API spec from the annotations to Swagger Documentation 2.0.

## Build docker image

Make sure that you have [docker](https://docs.docker.com/get-docker/).

Simple build:

```shell
docker build -t openobserve:latest-amd64 -f deploy/build/Dockerfile .
```

Multi-arch build

In order to build multi-arch builds you will need [buildx](https://docs.docker.com/buildx/working-with-buildx/) installed. You will need to pass the platform flag for the platform that you want to build.

```shell
docker build -t openobserve:latest-amd64 -f deploy/build/Dockerfile.tag.amd64 .
```

Please check folder deploy/build for docker files.

# Checks in CI pipeline

We check for following in CI pipeline for any pull requests.

1. Unit test code coverage for rust code.
   - If code coverage is less than 81% the CI tests will fail.
   - You can test coverage yourself by running `./coverage.sh`
   - We use [cargo-llvm-cov](https://github.com/taiki-e/cargo-llvm-cov) to generate code coverage report in JSON format.
   - Run `cargo llvm-cov --lcov --output-path lcov.info` to generate report and use [Coverage Gutters](https://marketplace.visualstudio.com/items?itemName=ryanluker.vscode-coverage-gutters) with VS code for visualizing code coverage.
   - You can also generate html report by using `cargo llvm-cov --html` to generate html report in target/debug/llvm-cov/html/index.html
1. Linting in Javascript for GUI
   - We run eslint for javascript and any linting failures will result in build failures.
   - You can test for linting failures by running `./eslint.sh` in web folder.

## How to contribute code

1. Fork the repository on github (e.g. awesomedev/openobserve)
1. Clone the repo from the forked repository ( e.g. awesomedev/openobserve) to your machine.
1. create a new branch locally.
1. Make the changes to code.
1. Push the code to your repo.
1. Create a PR
1. Make sure that the automatic CI checks pass for your PR.

## PR title should meet following criteria:

1. build: Changes that affect the build system or external dependencies (example scopes: gulp, broccoli, npm)
1. ci: Changes to our CI configuration files and scripts (example scopes: Travis, Circle, BrowserStack, SauceLabs)
1. docs: Documentation only changes
1. feat: A new feature
1. fix: A bug fix
1. perf: A code change that improves performance
1. refactor: A code change that neither fixes a bug nor adds a feature
1. style: Changes that do not affect the meaning of the code (white-space, formatting, missing semi-colons, etc)
1. test: Adding missing tests or correcting existing tests
