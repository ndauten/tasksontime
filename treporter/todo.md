# Tasks

Quick list:

Tooling:
- [x] Eliminate warnings 2025-11-4 ✅ 
- [x] add option to pull tokens from some default config location: maybe ~/.chronopulse? ✅ 

Collect+Summarize:
- [ ] All branches vs single default master for repositories


## Activities

## Bot Log

### 2025-11-04: Warning Elimination & Global Credential Management

**Completed Tasks:**

1. **✅ Eliminated All Compiler Warnings**
   - Fixed 11 library warnings and 15 binary warnings
   - Used `#[allow(dead_code)]` for intentionally unused but valuable code
   - Prefixed unused function parameters with underscore (e.g., `_raw_data`, `_template`)
   - Removed unused imports via `cargo fix --lib`
   - Result: Clean build with 0 warnings in both dev and release profiles
   - All 26 tests passing
   - Commit: `14ec71a`

2. **✅ Global Credential Management System**
   - Implemented `~/.chronopulse/config.toml` for global credentials
   - Created `credentials` module with `GlobalConfig` struct
   - Added `chronopulse setup` interactive wizard command
   - Added `chronopulse setup --show` to display current config (secrets masked)
   - Added `chronopulse setup --force` to overwrite existing config
   - Credential resolution priority: ENV vars → .env → ~/.chronopulse/config.toml
   - Supports GitLab, GitHub, and LLM providers (Ollama/OpenAI/Anthropic)
   - Created comprehensive 420+ line documentation: `docs/credential-management.md`
   - Added `dirs` crate dependency for cross-platform home directory detection
   - Commit: `cdc708e`

3. **✅ Repository Discovery Feature**
   - Implemented nested repository discovery with configurable options
   - Added `RepositoryDiscoveryConfig` with enabled, search_paths, max_depth, ignore_patterns
   - Created 10 comprehensive integration tests
   - All tests passing with proper isolation via tempfile
   - Commit: `36fffd3`

**Benefits:**
- Security: Credentials separated from project repos
- Convenience: Set credentials once, use across all projects
- Flexibility: Multiple credential sources with clear priority
- User Experience: Interactive setup wizard guides new users
- CI/CD Ready: Environment variables take precedence
- Code Quality: Clean codebase with zero warnings

**Files Modified:**
- `src/config.rs` - Added allow(dead_code) for future-use methods
- `src/credentials.rs` - Marked unused helper functions
- `src/generators/report.rs` - Fixed unused variable
- `src/llm.rs` - Marked unused methods and parameters
- `src/ollama.rs` - Marked health_check and generate_with_system
- `src/preprocessor.rs` - Marked unused struct fields
- `src/simple_pipeline.rs` - Marked unused prompt fields

**Next Steps:**
- Integrate global credentials into GitLab/GitHub collectors
- Consider implementing multi-branch repository analysis
- Explore additional LLM providers
