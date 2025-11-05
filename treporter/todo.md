# Tasks

Quick list:

Tooling:
- [x] Eliminate warnings 2025-11-4 ✅

Setup:
- [x] add option to pull tokens from some default config location: maybe ~/.chronopulse? ✅
- [x] enhance setup wizard with project configuration 2025-11-4 ✅

Collect+Summarize:
- [ ] All branches vs single default master for repositories


## Activities

## Bot Log

### 2025-11-04 (Latest): Enhanced Setup Wizard with Project Configuration

**Completed Tasks:**

1. **✅ Enhanced Setup Wizard - Complete Project Setup**
   - Extended `chronopulse setup` to handle both global credentials AND project configuration
   - Added interactive menu to choose: global only, project only, or both
   - New flags: `--global` (credentials only), `--project` (project only)
   - Project setup wizard includes:
     * Project name and description prompts
     * Data source selection (GitLab/GitHub/local files) with yes/no prompts
     * Repository discovery configuration (paths, max depth)
     * LLM provider selection (can inherit from global config)
     * Multi-line input for repository search paths
     * Automatic git repository detection
   - Generates complete, production-ready `config.toml` with user choices
   - Made `Config::find_git_root()` public for cross-module use
   - Added `prompt_yes_no()` helper for boolean prompts
   - Smart defaults pulled from global config when available
   - Updated README with "Complete Setup" section highlighting one-command setup
   - Commit: `41a4f73`

**Benefits:**
- **Complete onboarding in one command**: `chronopulse setup`
- **No manual config editing required** for basic setup
- **Interactive & user-friendly**: Clear prompts with sensible defaults
- **Flexible**: Can setup global, project, or both
- **Validates input**: Prevents invalid configurations
- **Smart defaults**: Reuses global config settings when appropriate

**User Experience:**
```bash
# Complete setup in one command
$ chronopulse setup
🔧 ChronoPulse Setup Wizard
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
What would you like to set up?
  1) Global credentials only (~/.chronopulse/config.toml)
  2) Project configuration only (./config.toml)
  3) Both global and project
Choice [3]:
```

**Next Steps:**
- Consider adding validation for tokens (test GitLab/GitHub API)
- Add option to import existing .env files
- Support updating existing configurations (not just overwrite)

---

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
