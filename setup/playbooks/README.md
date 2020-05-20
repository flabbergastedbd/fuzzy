## Requirements

### Host

- mkcert
- ansible

### Managed Machines

- Docker
- Docker SDK for Python (Master only)

## Usage

Following command will go ahead and deploy necessary infrastructure on machines.

``` bash
ansible-playbook -K -i hosts fuzzy.yml
```

If you only want to run specific tasks, use tags

``` bash
ansible-playbook -K -i hosts fuzzy.yml --tags "sync-files"
```

## Ports

### Master

- 3000  (Grafana interface)
- 5000  (Private docker registry used to distribute image)
- 12700 (XPC port for cli & workers)
