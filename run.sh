set -xe

[[ "$1" == "build" ]] && {
    shift
    docker build -t chainstate .
}

[[ "$1" == "publish-mainnet" ]] && {
    export SSH_HOST="root@xdai.enormous.cloud"
    docker save chainstate | bzip2 | ssh $SSH_HOST 'bunzip2 | docker load'
    ssh $SSH_HOST 'cd /opt/chainstate-xdai; docker rm -f chainstate-xdai; docker-compose up -d'
}