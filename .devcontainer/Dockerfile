# This is a docker file to specify our development environment
FROM rust:alpine

# Install python/pip
ENV PYTHONUNBUFFERED=1
RUN apk add --update --no-cache python3 && ln -sf python3 /usr/bin/python
RUN python3 -m ensurepip
RUN pip3 install --no-cache --upgrade pip setuptools

# Set up dependencies for our scripts.
# We don't want to bother with virtual environments just for some utility scripts.
RUN pip3 install mysql-connector-python