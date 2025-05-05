# Docker

## Build and run

### Make commands
- ```make build_node_image```: Build the ```qf-node-image``` image which using for development.
- ```make build_dev_image```: Build the ```qf-solochain-<arch>:latest``` image for development.
- ```make run_arm64```: Run the ```qf-solochain-arm64:latest``` image.
- ```make run_x86_64```: Run the ```qf-solochain-x86_64:latest``` image.
- ```make build_arm64```: Build the ```qf-solochain-arm64:latest``` image.
- ```make build_x86_64```: Build the ```qf-solochain-x86_64:latest``` image.

### Basic usage
For running the full node just run:
```bash
make run_arm64
```
or for x86_64 architecture:
```bash
make run_x86_64
```

### Development
We recommend to build first the ```qf-node-image``` because it builds a longer time and then use it to build the
```qf-solochain-<arch>:latest``` image.
```bash
make build_node_image
```
Or
You can
```bash
make build_dev_image
```
Then you can change the ```Dockerfile.dev```. The ```Dockerfile.dev``` is used to build the
```qf-solochain-<arch>:latest``` (default **arch** is **x86_64**) image but coping the compiled results from
```qf-node-image:latest``` (default **arch** is **x86_64**) and dont require to compile the ```qf-node-image``` again.
Run command with all parameters contained in the ```qf-node-start.sh``` script.
