FROM python:3.12-slim

ENV PYTHONUNBUFFERED=1 \
    PYTHONDONTWRITEBYTECODE=1

RUN groupadd -g 10001 appgroup && \
    useradd -u 10001 -g appgroup -s /bin/sh appuser

WORKDIR /app

RUN pip install --no-cache-dir chess

COPY wrapper.py /app/wrapper.py

CMD ["python", "-u", "wrapper.py"]