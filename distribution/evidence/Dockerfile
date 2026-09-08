FROM archlinux@sha256:694da1fce635e3a14d90751941b08f02c500e8724682f7a00768e7152251ec34
COPY distribution/evidence/pacman-snapshot.conf /etc/pacman.conf
RUN pacman -Syu --noconfirm python openssh \
    && pacman -Q python openssh \
    && pacman -Scc --noconfirm
