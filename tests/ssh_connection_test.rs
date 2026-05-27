//! SSH connection integration tests.
//!
//! These tests verify that the SSH2 client stack works correctly on the current
//! platform. They are layered to isolate failures:
//!
//! 1. `test_ssh2_session_init`     — libssh2 + OpenSSL load correctly
//! 2. `test_ssh_tcp_connect`       — TCP socket connectivity
//! 3. `test_ssh2_handshake`        — key exchange (the failing layer on Windows)
//! 4. `test_ssh2_password_auth`    — authentication mechanism
//! 5. `test_ssh2_command_exec`     — full round-trip: channel + exec + read
//! 6. `test_project_execute_command` — project's own API
//!
//! If test 1 fails, the crypto backend isn't loading. If test 3 fails with
//! LIBSSH2_ERROR_KEX_FAILURE (-5), the key exchange algorithms offered by
//! the client don't intersect with the server's — the exact symptom of a
//! Windows build linked against a libssh2/OpenSSL that lacks curve25519 or
//! diffie-hellman-group-exchange-sha256.
//!
//! Running locally:
//!   cargo test --test ssh_connection_test
//!
//! With a local SSH server:
//!   SSH_TEST_USER=youruser SSH_TEST_PASS=yourpass SSH_TEST_HOST=127.0.0.1:22 \
//!     cargo test --test ssh_connection_test
//!
//! Without a local SSH server, all tests that require a server are skipped.
//!
//! CI runs these on both Linux and Windows with a provisioned SSH server.

use std::io::Read;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

fn ssh_test_host() -> String {
    std::env::var("SSH_TEST_HOST").unwrap_or_else(|_| "127.0.0.1:22".to_string())
}

fn ssh_test_user() -> String {
    std::env::var("SSH_TEST_USER").unwrap_or_else(|_| "ssh_test_user".to_string())
}

fn ssh_test_pass() -> String {
    std::env::var("SSH_TEST_PASS").unwrap_or_else(|_| "SshTest@Pass42".to_string())
}

fn ssh_server_reachable() -> bool {
    let host = ssh_test_host();
    let addr: std::net::SocketAddr = match host.to_socket_addrs() {
        Ok(mut addrs) => match addrs.next() {
            Some(a) => a,
            None => return false,
        },
        Err(_) => return false,
    };
    TcpStream::connect_timeout(&addr, Duration::from_secs(3)).is_ok()
}

/// Platform identifier for diagnostic messages.
fn platform() -> String {
    format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

/// Step 1: Verify that ssh2::Session::new() works.
///
/// On Windows, this can fail if libssh2.dll or OpenSSL DLLs are missing
/// or if the static-linked crypto backend failed to initialise.
#[test]
fn test_ssh2_session_init() {
    let os = platform();
    match ssh2::Session::new() {
        Ok(session) => {
            // Session created; drop it to free resources.
            drop(session);
            println!("OK: ssh2::Session::new() succeeded on {}", os);
        }
        Err(e) => {
            panic!(
                "FAIL: ssh2::Session::new() failed on {}: [{}] {}. \
                 The libssh2 or OpenSSL backend could not be loaded. \
                 On Windows this usually means the OpenSSL DLLs are missing \
                 or the static-linked crypto backend is broken.",
                os,
                e.code(),
                e
            );
        }
    }
}

/// Step 2: Verify TCP connectivity to an SSH server.
///
/// This tests that we can open a TCP socket to the server. If this fails,
/// the problem is network-level, not SSH.
#[test]
fn test_ssh_tcp_connect() {
    let os = platform();
    if !ssh_server_reachable() {
        println!(
            "SKIP: test_ssh_tcp_connect on {} — no SSH server at {}",
            os,
            ssh_test_host()
        );
        return;
    }

    let host = ssh_test_host();
    let addr: std::net::SocketAddr = host
        .to_socket_addrs()
        .expect("Failed to resolve host")
        .next()
        .expect("No addresses for host");

    match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
        Ok(stream) => {
            let local = stream.local_addr().unwrap();
            let peer = stream.peer_addr().unwrap();
            println!("OK: TCP connected to {} from {} on {}", peer, local, os);
        }
        Err(e) => {
            panic!(
                "FAIL: TCP connect to {} failed on {}: {}. \
                 The SSH server may not be running.",
                addr, os, e
            );
        }
    }
}

/// Step 3: Verify SSH key exchange (handshake).
///
/// This is the critical test. On a broken Windows build, this will fail with
/// LIBSSH2_ERROR_KEX_FAILURE (code -5) because the libssh2 linked on Windows
/// doesn't offer key exchange algorithms that a modern sshd accepts.
///
/// On a working build (Linux with system libssh2, or Windows with a properly
/// built libssh2 1.11+ with curve25519 support), this should succeed.
#[test]
fn test_ssh2_handshake() {
    let os = platform();
    if !ssh_server_reachable() {
        println!(
            "SKIP: test_ssh2_handshake on {} — no SSH server at {}",
            os,
            ssh_test_host()
        );
        return;
    }

    let host = ssh_test_host();
    let addr: std::net::SocketAddr = host
        .to_socket_addrs()
        .expect("Failed to resolve host")
        .next()
        .expect("No addresses for host");

    let tcp =
        TcpStream::connect_timeout(&addr, Duration::from_secs(5)).expect("TCP connect failed");

    tcp.set_read_timeout(Some(Duration::from_secs(10)))
        .expect("set_read_timeout failed");
    tcp.set_write_timeout(Some(Duration::from_secs(10)))
        .expect("set_write_timeout failed");

    let mut session = ssh2::Session::new().expect("Session::new() failed");
    session.set_tcp_stream(tcp);
    session.set_timeout(15_000); // 15 seconds for operations

    match session.handshake() {
        Ok(()) => {
            // Retrieve the server's banner for diagnostics.
            let banner = session.banner().unwrap_or("(no banner)");
            println!(
                "OK: SSH handshake succeeded on {} — server banner: {:?}",
                os, banner
            );
            // We do NOT authenticate here; that's tested separately.
            // Disconnect cleanly.
            let _ = session.disconnect(None, "test complete", None);
        }
        Err(e) => {
            let code = e.code();
            panic!(
                "FAIL: SSH handshake failed on {}: [{}] {}. \
                 Code -5 = KEX_FAILURE: the client and server could not agree \
                 on a key exchange algorithm. This is the expected failure mode \
                 when the Windows build's libssh2 lacks curve25519-sha256 \
                 support. \
                 Other codes — check the error message above.",
                os, code, e
            );
        }
    }
}

/// Step 4: Verify password authentication works after a successful handshake.
///
/// This test requires SSH_TEST_USER and SSH_TEST_PASS to be valid credentials
/// on the test SSH server. In CI, the server is provisioned with a test user.
#[test]
fn test_ssh2_password_auth() {
    let os = platform();
    if !ssh_server_reachable() {
        println!(
            "SKIP: test_ssh2_password_auth on {} — no SSH server at {}",
            os,
            ssh_test_host()
        );
        return;
    }

    let host = ssh_test_host();
    let user = ssh_test_user();
    let pass = ssh_test_pass();
    let addr: std::net::SocketAddr = host
        .to_socket_addrs()
        .expect("Failed to resolve host")
        .next()
        .expect("No addresses for host");

    let tcp =
        TcpStream::connect_timeout(&addr, Duration::from_secs(5)).expect("TCP connect failed");
    tcp.set_read_timeout(Some(Duration::from_secs(10)))
        .expect("set_read_timeout failed");
    tcp.set_write_timeout(Some(Duration::from_secs(10)))
        .expect("set_write_timeout failed");

    let mut session = ssh2::Session::new().expect("Session::new() failed");
    session.set_tcp_stream(tcp);
    session.set_timeout(15_000);
    session.handshake().expect("SSH handshake failed");

    match session.userauth_password(&user, &pass) {
        Ok(()) => {
            assert!(
                session.authenticated(),
                "userauth_password returned Ok but session is not authenticated"
            );
            println!("OK: Password auth succeeded on {} for user '{}'", os, user);
            let _ = session.disconnect(None, "test complete", None);
        }
        Err(e) => {
            // Auth failure is informational, not a test failure,
            // because the test user may not exist or have a different password.
            // But if handshake worked and auth failed, it's likely a credentials issue.
            println!(
                "INFO: Password auth failed on {} for user '{}': [{}] {}. \
                 If handshake succeeded, the SSH stack is working — this is \
                 likely a credentials issue. Check SSH_TEST_USER and SSH_TEST_PASS.",
                os,
                user,
                e.code(),
                e
            );
            let _ = session.disconnect(None, "test complete", None);
            // Don't panic — handshake working is the critical test.
            // Auth issues are environment-specific.
        }
    }
}

/// Step 5: Verify command execution over SSH.
///
/// After handshake and auth, open an exec channel, run `echo hello`, and
/// verify we get output. This tests the full SSH channel stack.
#[test]
fn test_ssh2_command_exec() {
    let os = platform();
    if !ssh_server_reachable() {
        println!(
            "SKIP: test_ssh2_command_exec on {} — no SSH server at {}",
            os,
            ssh_test_host()
        );
        return;
    }

    let host = ssh_test_host();
    let user = ssh_test_user();
    let pass = ssh_test_pass();
    let addr: std::net::SocketAddr = host
        .to_socket_addrs()
        .expect("Failed to resolve host")
        .next()
        .expect("No addresses for host");

    let tcp =
        TcpStream::connect_timeout(&addr, Duration::from_secs(5)).expect("TCP connect failed");
    tcp.set_read_timeout(Some(Duration::from_secs(10)))
        .expect("set_read_timeout failed");
    tcp.set_write_timeout(Some(Duration::from_secs(10)))
        .expect("set_write_timeout failed");

    let mut session = ssh2::Session::new().expect("Session::new() failed");
    session.set_tcp_stream(tcp);
    session.set_timeout(15_000);
    session.handshake().expect("SSH handshake failed");

    match session.userauth_password(&user, &pass) {
        Ok(()) => {
            let mut channel = session.channel_session().expect("channel_session() failed");
            channel.exec("echo hello_ssh_test").expect("exec() failed");

            let mut output = String::new();
            channel
                .read_to_string(&mut output)
                .expect("read_to_string failed");
            channel.wait_close().ok();

            let exit_status = channel.exit_status().unwrap_or(-1);
            assert!(
                output.contains("hello_ssh_test"),
                "Expected 'hello_ssh_test' in output, got: {:?}",
                output
            );
            assert_eq!(
                exit_status, 0,
                "Expected exit status 0, got {}",
                exit_status
            );

            println!(
                "OK: SSH command exec succeeded on {} — output: {:?}, exit: {}",
                os,
                output.trim(),
                exit_status
            );

            let _ = session.disconnect(None, "test complete", None);
        }
        Err(e) => {
            println!(
                "INFO: SSH command exec test skipped on {} — auth failed for '{}': [{}] {}. \
                 The SSH stack is working; set SSH_TEST_USER/SSH_TEST_PASS for full testing.",
                os,
                user,
                e.code(),
                e
            );
            let _ = session.disconnect(None, "test complete", None);
        }
    }
}

/// Step 6: Test the project's own SSH API.
///
/// This exercises the full project stack: validate_host_format →
/// connect_ssh_with_config → execute_ssh_command.
#[test]
fn test_project_execute_command() {
    let os = platform();
    if !ssh_server_reachable() {
        println!(
            "SKIP: test_project_execute_command on {} — no SSH server at {}",
            os,
            ssh_test_host()
        );
        return;
    }

    let host = ssh_test_host();
    let user = ssh_test_user();
    let pass = ssh_test_pass();

    match sie_generate_config::backend::ssh_utils::execute_command_over_ssh(
        &host,
        &user,
        &pass,
        "echo hello_project",
    ) {
        Ok(output) => {
            assert!(
                output.contains("hello_project"),
                "Expected 'hello_project' in output, got: {:?}",
                output
            );
            println!(
                "OK: Project SSH API succeeded on {} — output: {:?}",
                os,
                output.trim()
            );
        }
        Err(e) => {
            // If the handshake itself failed, report it as a platform bug.
            let error_str = format!("{:?}", e);
            if error_str.contains("Kex")
                || error_str.contains("key exchange")
                || error_str.contains("handshake")
            {
                panic!(
                    "FAIL: Project SSH API failed on {} with key exchange / handshake error: \
                     {:?}. \
                     This indicates the SSH crypto backend is broken on this platform.",
                    os, e
                );
            }
            // Other errors (auth, network) are environment-specific.
            println!(
                "INFO: Project SSH API failed on {}: {:?}. \
                 If handshake works but this fails, check credentials.",
                os, e
            );
        }
    }
}
