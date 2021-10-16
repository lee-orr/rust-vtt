FROM gitpod/workspace-dotnet-vnc

RUN rustup default beta
RUN cd /home/gitpod
RUN wget https://downloads.tuxfamily.org/godotengine/3.3.4/Godot_v3.3.4-stable_x11.64.zip
RUN unzip Godot_v3.3.4-stable_x11.64.zip
