const std = @import("std");
pub fn build(b: *std.Build) void {
  const run_step = b.step("run", "Run");
  const test_step = b.step("test", "Test");
  _ = run_step; _ = test_step;
}
