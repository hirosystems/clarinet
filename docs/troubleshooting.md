---
# The default id is the same as the one defined below. so not needed
title: Troubleshooting
---

This page answers some common issues you may encounter when using Clarinet. Updates will be made to this page regularly as we receive feedback and comments from our developer community.

## I am unable to run Clarinet after installation.

> **_NOTE:_**
>
> The below steps are intended to address Windows users only.

If you cannot run Clarinet after you have installed it using the installer, perform the steps listed below.

1. Restart your shell/VSCode to ensure they have the updated path (the installer should have added the directory to the path).
2. If this does not resolve the issue, you must manually add the directory to your path.

To manually add the directory to your path:

1. Open "System Properties."
2. Click the "Environment Variables" button under "System variables."
3. Select "Path" and then click "Edit."
4. Click the "New" button and add "C:\Program Files\clarinet\bin."
5. Follow the on-screen prompts and click "OK" after each prompt.
6. Restart your shell/VSCode.

If you did not install Clarinet to your default directory, you would need to modify the path so that Clarinet points to the correct directory. 

## I am unable to start Devnet though my Docker is running.

When you run the `Clarinet integrate` command, you might experience the error: "unable to start Devnet: make sure that Docker is installed on this machine and running."

You can resolve the issue by creating a symlink of the docker.sock file. To do that, In your terminal, navigate to your Clarinet project directory and run the following command:

`sudo ln -s /Users/<your-username>/.docker/run/docker.sock /var/run/docker.sock` 

Now, run the command `clarinet integrate` to see the Devnet up and running.

## I am unable to set up docker on my Linux machine.

If you have trouble setting up docker on your Linux machine, follow the steps below:

1. To remove the current installation, follow the steps below:

    ```
    sudo apt-get purge -y docker-engine docker docker.io docker-ce
    sudo apt-get autoremove -y --purge docker-engine docker docker.io docker-ce
    sudo umount /var/lib/docker/
    sudo rm -rf /var/lib/docker /etc/docker
    sudo rm /etc/apparmor.d/docker
    sudo groupdel docker
    sudo rm -rf /var/run/docker.sock
    sudo rm -rf /usr/bin/docker-compose
    ```

2. Install docker-desktop by following the steps [here](https://docs.docker.com/desktop/install/ubuntu/#install-docker-desktop).
3. You will need to update the settings in the Clarinet project. You can do this by navigating to the Clarinet/components/clarinet-cli/examples/simple-nft/settings/Devnet.toml file. In the `[Devnet]` settings, add the following setting and replace `username` with your username:
        `docker_host = "/home/<username>/.docker/desktop/docker.sock"`
4. Save the `Devnet.toml` and run docker now.
