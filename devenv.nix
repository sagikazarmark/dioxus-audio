{ pkgs, lib, ... }:

{
  dagger.enable = true;
  env.DAGGER_X_RELEASE = "86d1d2f5791bcf3213d56903cfa81a3ba0abe54a";

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
