# NixOS Container configuration equivalent to Dockerfile.minimal
{ config, pkgs, ... }:

{
  system.stateVersion = "23.11";

  # SSH Configuration (equivalent to SSH setup in Dockerfile)
  services.openssh = {
    enable = true;
    settings = {
      PermitRootLogin = "yes";
      PasswordAuthentication = true;
    };
  };

  # Telegraf service (equivalent to telegraf installation and setup)
  services.telegraf = {
    enable = true;
    # Telegraf will run as telegraf user automatically
  };

  # InfluxDB service (equivalent to influxdb2 installation)
  services.influxdb2 = {
    enable = true;
  };

  # User configuration (equivalent to user creation in Dockerfile)
  users.users = {
    # Root user with password 'root'
    root = {
      password = "root";
    };
    
    # Test user equivalent to Dockerfile testuser setup
    testuser = {
      isNormalUser = true;
      password = "testpass";
      extraGroups = [ "wheel" "telegraf" ];  # wheel for sudo, telegraf for telegraf access
      home = "/home/testuser";
      createHome = true;
      shell = pkgs.bash;
    };
  };

  # Passwordless sudo for testuser (equivalent to sudoers.d setup)
  security.sudo = {
    enable = true;
    wheelNeedsPassword = false;
  };

  # System packages (equivalent to apt packages in Dockerfile)
  environment.systemPackages = with pkgs; [
    wget
    gnupg
    openssh
    sudo
    # ca-certificates and apt-transport-https equivalents are handled by NixOS
  ];

  # Networking configuration (equivalent to EXPOSE 22)
  networking.firewall = {
    enable = true;
    allowedTCPPorts = [ 22 ];
  };

  # Ensure telegraf directories and permissions are set up properly
  # (equivalent to the telegraf directory setup in Dockerfile)
  systemd.tmpfiles.rules = [
    "d /var/log/telegraf 0775 telegraf telegraf -"
    "f /var/log/telegraf/telegraf.log 0664 telegraf telegraf -"
  ];

  # Ensure telegraf user can access necessary directories
  users.groups.telegraf.members = [ "testuser" ];
}