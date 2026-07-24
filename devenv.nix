{ pkgs, lib, ... }:

{
  dagger.enable = true;
  env.DAGGER_X_RELEASE = "v1.0.0-beta.7";

  env.CC_wasm32_unknown_unknown = "${pkgs.llvmPackages.clang-unwrapped}/bin/clang";

  packages =
    with pkgs;
    [
      lld
      cargo-audit
      cargo-deny
      cargo-release
      cargo-watch
      dioxus-cli
      wasm-bindgen-cli_0_2_126
    ]
    ++ lib.optionals stdenv.isLinux [
      chromium
      chromedriver
    ];

  languages = {
    rust = {
      enable = true;
      channel = "stable";
      targets = [ "wasm32-unknown-unknown" ];
    };
    javascript = {
      enable = true;
      npm.enable = true;
    };
  };
}
