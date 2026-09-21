use std::sync::Arc;
use russh::client::{self, Handler};
use russh_sftp::client::SftpSession;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct ClientHandler;

impl Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::PublicKeyOrCertificate,
    ) -> Result<bool, Self::Error> {
        // Automatically trust remote host key for local game consoles
        Ok(true)
    }
}

pub struct RemoteSession {
    pub sftp: SftpSession,
    // Keep session alive
    _session: client::Handle<ClientHandler>,
}


#[derive(Debug, Clone)]
pub struct RemoteDirItem {
    pub name: String,
    pub is_dir: bool,
    pub size: u64,
}

impl RemoteSession {
    pub async fn connect(host: &str, port: u16, username: &str, password: &str) -> Result<Self, String> {
        let config = Arc::new(client::Config::default());
        let addr = format!("{}:{}", host, port);
        
        let sh = ClientHandler;
        let mut session = client::connect(config, addr, sh)
            .await
            .map_err(|e| format!("SSH 연결 실패 ({}): {}", host, e))?;

        let auth_res = session
            .authenticate_password(username, password)
            .await
            .map_err(|e| format!("인증 처리 오류: {}", e))?;

        if !matches!(auth_res, russh::client::AuthResult::Success) {
            return Err("아이디 또는 비밀번호가 올바르지 않습니다.".to_string());
        }

        let channel = session
            .channel_open_session()
            .await
            .map_err(|e| format!("SSH 채널 열기 실패: {}", e))?;

        channel
            .request_subsystem(true, "sftp")
            .await
            .map_err(|e| format!("SFTP 서브시스템 요청 실패: {}", e))?;

        let sftp = SftpSession::new(channel.into_stream())
            .await
            .map_err(|e| format!("SFTP 세션 생성 실패: {}", e))?;

        Ok(Self {
            sftp,
            _session: session,
        })
    }

    pub async fn path_exists(&self, path: &str) -> bool {
        self.sftp.try_exists(path).await.unwrap_or(false)
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<RemoteDirItem>, String> {
        let entries = self.sftp
            .read_dir(path)
            .await
            .map_err(|e| format!("디렉토리 조회 실패 ({}): {}", path, e))?;

        let mut items = Vec::new();
        for entry in entries {
            let name = entry.file_name();
            if name == "." || name == ".." {
                continue;
            }
            let mut is_dir = entry.file_type().is_dir() || entry.metadata().is_dir();
            if !is_dir && entry.file_type().is_symlink() {
                // Check if symlink target is a directory
                let sub_path = format!("{}/{}", path.trim_end_matches('/'), name);
                if let Ok(meta) = self.sftp.metadata(&sub_path).await {
                    if meta.file_type().is_dir() || meta.is_dir() {
                        is_dir = true;
                    }
                }
            }
            let size = entry.metadata().size.unwrap_or(0);
            items.push(RemoteDirItem {
                name,
                is_dir,
                size,
            });
        }
        Ok(items)
    }

    pub async fn read_file_string(&self, path: &str) -> Result<String, String> {
        let bytes = self.read_file_bytes(path).await?;
        String::from_utf8(bytes).map_err(|e| format!("UTF-8 디코딩 실패 ({}): {}", path, e))
    }

    pub async fn read_file_bytes(&self, path: &str) -> Result<Vec<u8>, String> {
        self.sftp
            .read(path)
            .await
            .map_err(|e| format!("파일 읽기 실패 ({}): {}", path, e))
    }

    pub async fn write_file_bytes(&self, path: &str, data: &[u8]) -> Result<(), String> {
        let normalized = path.replace('\\', "/");
        if let Some(pos) = normalized.rfind('/') {
            let parent_dir = &normalized[..pos];
            if !parent_dir.is_empty() {
                self.create_dir_all(parent_dir).await?;
            }
        }

        let mut file = self.sftp
            .create(path)
            .await
            .map_err(|e| format!("파일 생성 실패 ({}): {}", path, e))?;

        file.write_all(data)
            .await
            .map_err(|e| format!("파일 쓰기 실패 ({}): {}", path, e))?;

        file.close()
            .await
            .map_err(|e| format!("파일 닫기 실패 ({}): {}", path, e))?;

        Ok(())
    }

    pub async fn write_file_from_local<F>(
        &self,
        local_path: &std::path::Path,
        remote_path: &str,
        mut on_progress: F,
    ) -> Result<u64, String>
    where
        F: FnMut(u64),
    {
        let normalized = remote_path.replace('\\', "/");
        if let Some(pos) = normalized.rfind('/') {
            let parent_dir = &normalized[..pos];
            if !parent_dir.is_empty() {
                self.create_dir_all(parent_dir).await?;
            }
        }

        let mut local_file = tokio::fs::File::open(local_path)
            .await
            .map_err(|e| format!("로컬 파일 열기 실패 ({}): {}", local_path.display(), e))?;

        let mut remote_file = self.sftp
            .create(&normalized)
            .await
            .map_err(|e| format!("원격 파일 생성 실패 ({}): {}", normalized, e))?;

        let mut buffer = vec![0u8; 256 * 1024]; // 256KB buffer for high throughput
        let mut total_written = 0u64;

        loop {
            let bytes_read = local_file
                .read(&mut buffer)
                .await
                .map_err(|e| format!("로컬 파일 읽기 실패: {}", e))?;

            if bytes_read == 0 {
                break;
            }

            remote_file
                .write_all(&buffer[..bytes_read])
                .await
                .map_err(|e| format!("원격 파일 쓰기 실패: {}", e))?;

            total_written += bytes_read as u64;
            on_progress(total_written);
        }

        remote_file
            .close()
            .await
            .map_err(|e| format!("원격 파일 닫기 실패: {}", e))?;

        Ok(total_written)
    }

    pub async fn exec_command(&self, cmd: &str) -> Result<String, String> {
        let mut channel = self._session
            .channel_open_session()
            .await
            .map_err(|e| format!("SSH 세션 채널 열기 실패: {}", e))?;

        channel
            .exec(true, cmd)
            .await
            .map_err(|e| format!("명령어 실행 요청 실패: {}", e))?;

        let mut output = Vec::new();
        while let Some(msg) = channel.wait().await {
            match msg {
                russh::ChannelMsg::Data { data } => {
                    output.extend_from_slice(&data);
                }
                russh::ChannelMsg::ExtendedData { data, .. } => {
                    output.extend_from_slice(&data);
                }
                russh::ChannelMsg::Eof => break,
                russh::ChannelMsg::Close => break,
                _ => {}
            }
        }

        String::from_utf8(output).map_err(|e| format!("출력 디코딩 실패: {}", e))
    }

    pub async fn create_dir_all(&self, path: &str) -> Result<(), String> {
        // Recursively create directories if needed
        let parts: Vec<&str> = path.split('/').filter(|p| !p.is_empty()).collect();
        let mut current = String::new();
        for part in parts {
            current.push('/');
            current.push_str(part);
            if !self.path_exists(&current).await {
                if let Err(e) = self.sftp.create_dir(&current).await {
                    if !self.path_exists(&current).await {
                        return Err(format!("디렉토리 생성 실패 ({}): {}", current, e));
                    }
                }
            }
        }
        Ok(())
    }
}
