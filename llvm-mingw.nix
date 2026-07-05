final: prev:
let
  version = "20260616";
  arch = builtins.head (prev.lib.splitString "-" prev.stdenv.hostPlatform.system);

  url = "https://github.com/mstorsjo/llvm-mingw/releases/download/${version}/llvm-mingw-${version}-ucrt-ubuntu-22.04-${arch}.tar.xz";
  hash = {
    x86_64 = "sha256:534b92e067b22a6b4441f48ae9240a3341b17825d04d577eab0cf85c44b4deda";
    aarch64 = "sha256:e7e5d135d93d3f2a3beaaea633a5b0e66ac75391a53feae654391913dd76102b";
  };
in
{
  llvm-mingw = prev.stdenv.mkDerivation {
    pname = "llvm-mingw";
    inherit version;

    src = prev.fetchurl {
      inherit url;
      hash = hash."${arch}";
    };

    nativeBuildInputs = [ prev.autoPatchelfHook ];
    buildInputs = with prev; [
      stdenv.cc.cc.lib
      zlib
      zstd
    ];

    dontConfigure = true;
    dontBuild = true;

    installPhase = ''
      mkdir -p $out
      cp -r ./* $out/

      rm -f $out/bin/lldb*
      rm -f $out/lib/liblldb*
    '';
  };
}
