FROM node:12

COPY package.json /project/

RUN cd /project \
    && npm install \
    && chown -R node .

#    && npm install tailwindcss@latest autoprefixer@latest \
#    && npx tailwindcss init \

WORKDIR /project
USER node
