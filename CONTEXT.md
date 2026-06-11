# CXC — Glossary

## CXC (Codex Cross-Connect)

The project name and CLI command name. A meta-configuration tool that manages API relay endpoint configurations for AI coding tools (Codex, Claude, etc.). Users invoke it as `cxc` in the terminal to quickly add, test, and switch API relay endpoints for their AI tools.

## Provider（中转站）

A named API relay endpoint configuration. Each Provider consists of a `name`, `base_url`, `api_key`, and `model`. Represents one API proxy or relay service that an AI coding tool (Codex, Claude) can be pointed at.

## Target Tool（目标工具）

An AI coding tool whose configuration CXC manages. Each Target Tool has a known config file path and format. MVP targets: Codex. Future: Claude. CXC writes Provider details into the Target Tool's config file when the user switches providers.

## Active Provider

The Provider currently configured for a given Target Tool. When the user switches providers, CXC updates the Target Tool's config file to point at the new Provider's endpoint.

## Connectivity Test

A lightweight, real API call (minimal chat completion request) sent to a Provider's endpoint to verify that the `base_url`, `api_key`, and `model` are valid and the service is reachable. Not a synthetic ping — it exercises the actual inference path.
