---
title: FAQ's
---

#### **After installing Clarinet on Windows with the installer, how can I run Clarinet?**

- First, restart your shell/VSCode to ensure they have the updated Path (the installer should have added the directory to the path).
- If that does not work, manually add the directory to your Path by following the steps below:

    -  Open "System Properties", select "Environment Variables" button, under "System variables", select "Path" and hit "Edit". 
    - Press the "New" button and add "C:\Program Files\clarinet\bin", then press Ok, Ok, Ok. 
    - Finally, restart your shell or VSCode.
    - If you did not install to the default directory, modify the path accordingly.

Below are some screenshots to help with this:

![FAQ - 2](images/clarinet-faq-1.png)

![FAQ - 2](images/clarinet-faq-2.png)

![FAQ - 2](images/clarinet-faq-3.png)

![FAQ - 2](images/clarinet-faq-4.png)

#### **The command `clarinet integrate` is not working. How can I fix it?**

Clarinet integrate uses Docker to run Bitcoin, Stacks nodes, and other services. To run the `clarinet integrate` feature,  you'll need to [install Docker](https://docs.docker.com/get-docker/). If you're on Linux, ensure you are set up in [rootless mode](https://docs.docker.com/engine/security/rootless/).
