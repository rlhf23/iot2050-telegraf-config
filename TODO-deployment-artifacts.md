# TODO: Update deployment.rs to use GitHub Artifacts

## Current State
- ✅ GitHub Actions builds aarch64 binary using `cross` tool
- ✅ Binary uploaded as GitHub Artifact (90 day retention)
- x  Binary removed from git repo (no more bloat) (no i left it in actually)
- ✅ CI uses Dockerfile.build to build from source (x86_64)
- ✅ IoT device uses Dockerfile.prebuilt (expects binary in place)

## Problem
`deployment.rs` currently downloads repo tarball and runs `docker compose build`.
This expects the binary to be in `docker/api-service/docker-api-service`, but it's
no longer committed to the repo.

## Solution Options

### Option 1: Download Latest Artifact (Recommended)
Use GitHub API to download the latest successful artifact:

```rust
// In deployment.rs, before docker compose build:
async fn download_api_service_binary(&self, session: &Session) -> Result<()> {
    // 1. Get latest workflow run for the branch
    let url = format!(
        "https://api.github.com/repos/rlhf23/iot2050-telegraf-config/actions/workflows/api-service-build.yml/runs?branch={}&status=success&per_page=1",
        self.branch
    );
    
    // 2. Get artifact download URL from the run
    // 3. Download artifact (requires GitHub token or public repo)
    // 4. Extract binary to ~/monitoring/docker/api-service/docker-api-service
    // 5. chmod +x
}
```

**Pros:**
- Always gets latest binary for the branch
- No manual steps
- Works with any branch

**Cons:**
- Requires GitHub API token (or repo must be public)
- More complex code
- Artifacts expire after 90 days

### Option 2: Use GitHub Releases
Create releases with attached binaries:

```rust
async fn download_from_release(&self, session: &Session) -> Result<()> {
    // Download from: https://github.com/rlhf23/iot2050-telegraf-config/releases/latest/download/docker-api-service
}
```

**Pros:**
- Simple, stable URLs
- No expiration
- No auth needed for public repos

**Cons:**
- Requires manual release creation (or automate with tags)
- Only works for tagged versions

### Option 3: Build on Device
Just use Dockerfile.build on the device:

**Pros:**
- No artifact download needed
- Always fresh build

**Cons:**
- Slow on IoT device (Rust compilation)
- May run out of memory

## Recommended Approach

**Phase 1 (Quick Fix):**
- Use Option 2 (Releases) for stable versions
- Update workflow to create release on tags
- deployment.rs downloads from latest release

**Phase 2 (Full Solution):**
- Implement Option 1 for branch-specific deployments
- Add GitHub token to deployment config
- Download artifact matching the branch being deployed

## Implementation Checklist

- [ ] Update api-service-build.yml to create releases on tags
- [ ] Add download_binary() function to deployment.rs
- [ ] Test with a release tag (v0.1.0)
- [ ] Add error handling for missing artifacts
- [ ] Document in README how to deploy specific versions
- [ ] Consider caching downloaded binaries locally

## Files to Modify

1. `.github/workflows/api-service-build.yml` - Already has release step, just needs testing
2. `src/backend/deployment.rs` - Add binary download logic
3. `docker/api-service/Dockerfile.prebuilt` - Maybe add fallback to build from source?

## Testing Plan

1. Create a test tag: `git tag v0.1.0-test && git push --tags`
2. Verify artifact is attached to release
3. Test deployment.rs downloads and uses it
4. Verify docker compose build works with downloaded binary

## Notes

- Current artifact retention: 90 days
- Release binaries: permanent
- Need to handle both scenarios (artifact vs release)
- Consider adding version check/validation
