POSTGRES_IMAGE=postgres:alpine
PGDATA=/var/lib/postgresql/data

CONTAINER_NAME=fuzzy_db
MOUNT_POINT=/home/tunnelshade/workspace/mounts/postgres_fuzzy

start:
	docker run --env-file env -d -p 127.0.0.1:5432:5432 --name $(CONTAINER_NAME) -v $(MOUNT_POINT):$(PGDATA) $(POSTGRES_IMAGE)

stop:
	docker stop $(CONTAINER_NAME)

prune:
	docker rm $(CONTAINER_NAME)
	sudo rm -rf $(MOUNT_POINT)
