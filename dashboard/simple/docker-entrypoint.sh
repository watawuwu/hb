#!/bin/sh

envsubst < /usr/local/openresty/nginx/conf/nginx.conf.template > /usr/local/openresty/nginx/conf/nginx.conf
exec nginx -g 'daemon off;'
