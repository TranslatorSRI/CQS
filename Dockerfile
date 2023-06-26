FROM rustlang/rust:nightly-bullseye

ARG USERNAME=nru
ARG USER_UID=1000
ARG USER_GID=1000

RUN groupadd --gid $USER_GID $USERNAME && useradd --uid $USER_UID --gid $USER_GID --system --create-home -m $USERNAME
RUN chown -R $USER_UID:$USER_GID /home/$USERNAME
RUN mkdir /home/$USERNAME/.cargo
RUN chown -R $USER_UID:$USER_GID /home/$USERNAME/.cargo
RUN chmod -R 755 /home/$USERNAME

RUN apt install bash

USER $USERNAME
RUN mkdir /home/$USERNAME/cqs
WORKDIR /home/$USERNAME/cqs

COPY . .

EXPOSE 8000

CMD ["/bin/bash", "-c", "/home/nru/cqs/main.sh"]

