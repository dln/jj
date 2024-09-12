// Copyright 2022 The Jujutsu Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
// https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::path::Path;

use itertools::Itertools as _;

use crate::common::TestEnvironment;

#[test]
fn test_concurrent_operation_divergence() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "message 1"]);
    test_env.jj_cmd_ok(
        &repo_path,
        &["describe", "-m", "message 2", "--at-op", "@-"],
    );

    // "--at-op=@" disables op heads merging, and prints head operation ids.
    let stderr = test_env.jj_cmd_failure(&repo_path, &["op", "log", "--at-op=@"]);
    insta::assert_snapshot!(stderr, @r#"
    Error: The "@" expression resolved to more than one operation
    Hint: Try specifying one of the operations by ID: ab036b52ca3c, 7e278fc1ebfa
    "#);

    // "op log --at-op" should work without merging the head operations
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "--at-op=7e278fc1ebfa"]);
    insta::assert_snapshot!(stdout, @r#"
    @  7e278fc1ebfa test-username@host.example.com 2001-02-03 04:05:09.000 +07:00 - 2001-02-03 04:05:09.000 +07:00
    │  describe commit 230dd059e1b059aefc0da06a2e5a7dbf22362f22
    │  args: jj describe -m 'message 2' --at-op @-
    ○  abf67f7f832a test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  8d6c5d6e5731 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  initialize repo
    ○  000000000000 root()
    "#);

    // We should be informed about the concurrent modification
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["log", "-T", "description"]);
    insta::assert_snapshot!(stdout, @r###"
    ○  message 2
    │ @  message 1
    ├─╯
    ◆
    "###);
    insta::assert_snapshot!(stderr, @r###"
    Concurrent modification detected, resolving automatically.
    "###);
}

#[test]
fn test_concurrent_operations_auto_rebase() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    std::fs::write(repo_path.join("file"), "contents").unwrap();
    test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "initial"]);
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log"]);
    insta::assert_snapshot!(stdout, @r#"
    @  789bfb61c764 test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  describe commit 4e8f9d2be039994f589b4e57ac5e9488703e604d
    │  args: jj describe -m initial
    ○  3d3a84a1d91d test-username@host.example.com 2001-02-03 04:05:08.000 +07:00 - 2001-02-03 04:05:08.000 +07:00
    │  snapshot working copy
    │  args: jj describe -m initial
    ○  abf67f7f832a test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  add workspace 'default'
    ○  8d6c5d6e5731 test-username@host.example.com 2001-02-03 04:05:07.000 +07:00 - 2001-02-03 04:05:07.000 +07:00
    │  initialize repo
    ○  000000000000 root()
    "#);
    let op_id_hex = stdout[3..15].to_string();

    test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "rewritten"]);
    test_env.jj_cmd_ok(
        &repo_path,
        &["new", "--at-op", &op_id_hex, "-m", "new child"],
    );

    // We should be informed about the concurrent modification
    let (stdout, stderr) = get_log_output_with_stderr(&test_env, &repo_path);
    insta::assert_snapshot!(stdout, @r###"
    ○  db141860e12c2d5591c56fde4fc99caf71cec418 new child
    @  07c3641e495cce57ea4ca789123b52f421c57aa2 rewritten
    ◆  0000000000000000000000000000000000000000
    "###);
    insta::assert_snapshot!(stderr, @r###"
    Concurrent modification detected, resolving automatically.
    Rebased 1 descendant commits onto commits rewritten by other operation
    "###);
}

#[test]
fn test_concurrent_operations_wc_modified() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");

    std::fs::write(repo_path.join("file"), "contents\n").unwrap();
    test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "initial"]);
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log"]);
    let op_id_hex = stdout[3..15].to_string();

    test_env.jj_cmd_ok(
        &repo_path,
        &["new", "--at-op", &op_id_hex, "-m", "new child1"],
    );
    test_env.jj_cmd_ok(
        &repo_path,
        &["new", "--at-op", &op_id_hex, "-m", "new child2"],
    );
    std::fs::write(repo_path.join("file"), "modified\n").unwrap();

    // We should be informed about the concurrent modification
    let (stdout, stderr) = get_log_output_with_stderr(&test_env, &repo_path);
    insta::assert_snapshot!(stdout, @r###"
    @  4eadcf3df11f46ef3d825c776496221cc8303053 new child1
    │ ○  68119f1643b7e3c301c5f7c2b6c9bf4ccba87379 new child2
    ├─╯
    ○  2ff7ae858a3a11837fdf9d1a76be295ef53f1bb3 initial
    ◆  0000000000000000000000000000000000000000
    "###);
    insta::assert_snapshot!(stderr, @r###"
    Concurrent modification detected, resolving automatically.
    "###);
    let stdout = test_env.jj_cmd_success(&repo_path, &["diff", "--git"]);
    insta::assert_snapshot!(stdout, @r###"
    diff --git a/file b/file
    index 12f00e90b6..2e0996000b 100644
    --- a/file
    +++ b/file
    @@ -1,1 +1,1 @@
    -contents
    +modified
    "###);

    // The working copy should be committed after merging the operations
    let stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "-Tdescription"]);
    insta::assert_snapshot!(stdout, @r###"
    @  snapshot working copy
    ○    reconcile divergent operations
    ├─╮
    ○ │  new empty commit
    │ ○  new empty commit
    ├─╯
    ○  describe commit 506f4ec3c2c62befa15fabc34ca9d4e6d7bef254
    ○  snapshot working copy
    ○  add workspace 'default'
    ○  initialize repo
    ○
    "###);
}

#[test]
fn test_concurrent_snapshot_wc_reloadable() {
    let test_env = TestEnvironment::default();
    test_env.jj_cmd_ok(test_env.env_root(), &["git", "init", "repo"]);
    let repo_path = test_env.env_root().join("repo");
    let op_heads_dir = repo_path
        .join(".jj")
        .join("repo")
        .join("op_heads")
        .join("heads");

    std::fs::write(repo_path.join("base"), "").unwrap();
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "initial"]);

    // Create new commit and checkout it.
    std::fs::write(repo_path.join("child1"), "").unwrap();
    test_env.jj_cmd_ok(&repo_path, &["commit", "-m", "new child1"]);

    let template = r#"id ++ "\n" ++ description ++ "\n" ++ tags"#;
    let op_log_stdout = test_env.jj_cmd_success(&repo_path, &["op", "log", "-T", template]);
    insta::assert_snapshot!(op_log_stdout, @r#"
    @  db25b8d38092e386259980770f8e66fd7e715cdd0dcc9ea333e7323f21cc26817f79f20328bb9a220cc1982da79692a5edebe0c700bf73afb2691e38ac90c1cd
    │  commit 554d22b2c43c1c47e279430197363e8daabe2fd6
    │  args: jj commit -m 'new child1'
    ○  e900a9ad48acb12eb082274cf300935546cff3353162b1208285cc991371e1d07a5afd9f0fec34de5092f062c3453230bd374dd368e55037ed2e2d412f91e1f8
    │  snapshot working copy
    │  args: jj commit -m 'new child1'
    ○  6cc2c241d2749fecf83cc7c9d5bb33833f86eaac455e0ee9ecd2b2a3319771177cf892f2b8488fb20ded2ff3127e6a4e115d5f848323bfa30fddfba795100171
    │  commit de71e09289762a65f80bb1c3dae2a949df6bcde7
    │  args: jj commit -m initial
    ○  41ba61bf5fb731a744052caa57dc0568ea1c5c9f4e0ef82d190b75a93501af158c1040f03ea6da46738814a25927dc97b40f101a055138fffdce398e70d07a6e
    │  snapshot working copy
    │  args: jj commit -m initial
    ○  abf67f7f832a282fe7c13bf77be5559cd52af19674b544d9172c3fec49377a1d177974c9600088418a07671f79712ec0507096029a724591047adfdacf430ba4
    │  add workspace 'default'
    ○  8d6c5d6e5731c662a50c53599987ad7db61a8d0719f1761788fa88ef68e9d869de04b1f47cc1a9319964d8edd4e40e670657902510c7bc2de5ded70525dc0975
    │  initialize repo
    ○  00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000

    "#);
    let op_log_lines = op_log_stdout.lines().collect_vec();
    let current_op_id = op_log_lines[0].split_once("  ").unwrap().1;
    let previous_op_id = op_log_lines[6].split_once("  ").unwrap().1;

    // Another process started from the "initial" operation, but snapshots after
    // the "child1" checkout has been completed.
    std::fs::rename(
        op_heads_dir.join(current_op_id),
        op_heads_dir.join(previous_op_id),
    )
    .unwrap();
    std::fs::write(repo_path.join("child2"), "").unwrap();
    let (stdout, stderr) = test_env.jj_cmd_ok(&repo_path, &["describe", "-m", "new child2"]);
    insta::assert_snapshot!(stdout, @"");
    insta::assert_snapshot!(stderr, @r###"
    Working copy now at: kkmpptxz 1795621b new child2
    Parent commit      : rlvkpnrz 86f54245 new child1
    "###);

    // Since the repo can be reloaded before snapshotting, "child2" should be
    // a child of "child1", not of "initial".
    let template = r#"commit_id ++ " " ++ description"#;
    let stdout = test_env.jj_cmd_success(&repo_path, &["log", "-T", template, "-s"]);
    insta::assert_snapshot!(stdout, @r###"
    @  1795621b54f4ebb435978b65d66bc0f90d8f20b6 new child2
    │  A child2
    ○  86f54245e13f850f8275b5541e56da996b6a47b7 new child1
    │  A child1
    ○  84f07f6bca2ffeddac84a8b09f60c6b81112375c initial
    │  A base
    ◆  0000000000000000000000000000000000000000
    "###);
}

fn get_log_output_with_stderr(test_env: &TestEnvironment, cwd: &Path) -> (String, String) {
    let template = r#"commit_id ++ " " ++ description"#;
    test_env.jj_cmd_ok(cwd, &["log", "-T", template])
}
