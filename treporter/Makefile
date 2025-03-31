# SPDX-License-Identifier: MIT
# 
# MIT License
# 
# © 2024 Nathan Dautenhahn & Serenitix LLC
# 
# Permission is hereby granted, free of charge, to any person obtaining a copy
# of this software and associated documentation files (the "Software"), to deal
# in the Software without restriction, including without limitation the rights
# to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
# copies of the Software, and to permit persons to whom the Software is
# furnished to do so, subject to the following conditions:
# 
# The above copyright notice and this permission notice shall be included in all
# copies or substantial portions of the Software.

# Info for the Rust build tool: name must match Cargo.toml
PROJECT_NAME 	:= cpm_if_yaml_validator
BUILD_DIR 		:= target/release
BINARY 			:= $(BUILD_DIR)/$(PROJECT_NAME)
SRC_DIR 		:= src

.PHONY: all build run test clean install setup-rust

all: setup build test example

#--- use automated testing framework to test
test: setup
	@cargo test

#--- build the tool
build: setup
	@cargo build --release

#--- setup recipe for any env or dependency requirements
setup: setup-rust

#--- Install Rust if it is not installed
setup-rust:
	@which rustc > /dev/null 2>&1 || ( 			\
		echo "Rust not found, installing..." 	\
		&& curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y \
		)
	@echo "Rust is installed: $$(rustc --version)"

clean:
	cargo clean

#--- Install the tool into the PATH
install: build
	install -m 755 $(BINARY) /usr/local/bin/$(PROJECT_NAME)

uninstall:
	rm -f /usr/local/bin/$(PROJECT_NAME)
