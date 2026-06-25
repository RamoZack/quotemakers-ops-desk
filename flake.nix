{
  description = "QuoteMakers Ops Desk development environment";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs = { nixpkgs, ... }:
    let
      system = "x86_64-linux";
      pkgs = import nixpkgs { inherit system; };
    in {
      devShells.${system}.default = pkgs.mkShell {
        packages = [
          pkgs.cargo
          pkgs.rustc
          pkgs.rustfmt
          pkgs.clippy
          pkgs.postgresql
          pkgs.nodejs_22
        ];

        shellHook = ''
          export DATABASE_URL="''${DATABASE_URL:-postgres://omarm@localhost/quotemakers_ops_desk?host=/var/run/postgresql}"
          echo "QuoteMakers Ops Desk dev shell"
        '';
      };
    };
}
