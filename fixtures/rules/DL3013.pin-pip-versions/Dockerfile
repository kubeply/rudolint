FROM python:3.12-slim
RUN pip install flask
RUN pip install flask==3.0.0
RUN pip3 install --no-cache-dir requests==2.32.3 click
RUN pip install -r requirements.txt
RUN pip install --index-url https://example.invalid/simple flask==3.0.0
RUN pip install -f /wheels flask==3.0.0
RUN pip install --no-binary :all: flask==3.0.0
RUN pip install -e .
RUN pip install --editable git+https://example.invalid/project.git#egg=project
RUN echo pip install flask
RUN cd /tmp && pip install build==1.2.2 wheel
