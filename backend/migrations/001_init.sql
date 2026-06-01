CREATE TABLE IF NOT EXISTS roles (
  role_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  role_name VARCHAR(50) NOT NULL UNIQUE,
  description VARCHAR(255) NULL
);

CREATE TABLE IF NOT EXISTS users (
  user_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  role_id BIGINT NOT NULL,
  username VARCHAR(64) NOT NULL UNIQUE,
  password_hash VARCHAR(255) NOT NULL,
  email VARCHAR(128) NULL,
  department VARCHAR(128) NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_users_role FOREIGN KEY (role_id) REFERENCES roles(role_id)
);

CREATE TABLE IF NOT EXISTS categories (
  category_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  category_name VARCHAR(100) NOT NULL UNIQUE,
  description VARCHAR(255) NULL
);

CREATE TABLE IF NOT EXISTS document_files (
  file_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  object_key VARCHAR(255) NOT NULL UNIQUE,
  original_name VARCHAR(255) NOT NULL,
  mime_type VARCHAR(128) NOT NULL,
  file_size BIGINT NOT NULL,
  sha256 VARCHAR(64) NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE IF NOT EXISTS documents (
  document_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  category_id BIGINT NOT NULL,
  creator_id BIGINT NOT NULL,
  current_version_id BIGINT NULL,
  current_version_no VARCHAR(20) NOT NULL,
  title VARCHAR(200) NOT NULL,
  summary TEXT NULL,
  status VARCHAR(20) NOT NULL,
  source_file_id BIGINT NULL,
  published_at DATETIME NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_documents_category FOREIGN KEY (category_id) REFERENCES categories(category_id),
  CONSTRAINT fk_documents_creator FOREIGN KEY (creator_id) REFERENCES users(user_id),
  CONSTRAINT fk_documents_source_file FOREIGN KEY (source_file_id) REFERENCES document_files(file_id)
);

CREATE TABLE IF NOT EXISTS document_versions (
  version_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  document_id BIGINT NOT NULL,
  version_no VARCHAR(20) NOT NULL,
  title VARCHAR(200) NOT NULL,
  content LONGTEXT NOT NULL,
  summary TEXT NULL,
  change_note VARCHAR(255) NOT NULL,
  source_file_id BIGINT NULL,
  is_published_snapshot TINYINT(1) NOT NULL DEFAULT 0,
  created_by BIGINT NOT NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uk_document_version (document_id, version_no),
  CONSTRAINT fk_versions_document FOREIGN KEY (document_id) REFERENCES documents(document_id),
  CONSTRAINT fk_versions_source_file FOREIGN KEY (source_file_id) REFERENCES document_files(file_id),
  CONSTRAINT fk_versions_created_by FOREIGN KEY (created_by) REFERENCES users(user_id)
);

ALTER TABLE documents
  ADD CONSTRAINT fk_documents_current_version FOREIGN KEY (current_version_id) REFERENCES document_versions(version_id);

CREATE TABLE IF NOT EXISTS tags (
  tag_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  tag_name VARCHAR(50) NOT NULL UNIQUE,
  description VARCHAR(255) NULL
);

CREATE TABLE IF NOT EXISTS document_tags (
  id BIGINT PRIMARY KEY AUTO_INCREMENT,
  document_id BIGINT NOT NULL,
  tag_id BIGINT NOT NULL,
  UNIQUE KEY uk_document_tag (document_id, tag_id),
  CONSTRAINT fk_document_tags_document FOREIGN KEY (document_id) REFERENCES documents(document_id),
  CONSTRAINT fk_document_tags_tag FOREIGN KEY (tag_id) REFERENCES tags(tag_id)
);

CREATE TABLE IF NOT EXISTS read_records (
  read_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  user_id BIGINT NOT NULL,
  document_id BIGINT NOT NULL,
  read_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_read_user FOREIGN KEY (user_id) REFERENCES users(user_id),
  CONSTRAINT fk_read_document FOREIGN KEY (document_id) REFERENCES documents(document_id)
);

CREATE TABLE IF NOT EXISTS favorite_records (
  favorite_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  user_id BIGINT NOT NULL,
  document_id BIGINT NOT NULL,
  favorite_time DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uk_favorite_user_document (user_id, document_id),
  CONSTRAINT fk_favorite_user FOREIGN KEY (user_id) REFERENCES users(user_id),
  CONSTRAINT fk_favorite_document FOREIGN KEY (document_id) REFERENCES documents(document_id)
);

CREATE TABLE IF NOT EXISTS questions (
  question_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  user_id BIGINT NOT NULL,
  question_text TEXT NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'answered',
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_questions_user FOREIGN KEY (user_id) REFERENCES users(user_id)
);

CREATE TABLE IF NOT EXISTS answers (
  answer_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  question_id BIGINT NOT NULL,
  answer_text LONGTEXT NOT NULL,
  confidence_score DECIMAL(5,2) NULL,
  model_name VARCHAR(100) NOT NULL,
  status VARCHAR(20) NOT NULL,
  latency_ms INT NULL,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_answers_question FOREIGN KEY (question_id) REFERENCES questions(question_id)
);

CREATE TABLE IF NOT EXISTS faq_items (
  faq_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  document_id BIGINT NOT NULL,
  question VARCHAR(255) NOT NULL,
  answer TEXT NOT NULL,
  status VARCHAR(20) NOT NULL DEFAULT 'active',
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  CONSTRAINT fk_faq_document FOREIGN KEY (document_id) REFERENCES documents(document_id)
);

CREATE TABLE IF NOT EXISTS document_segments (
  segment_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  version_id BIGINT NOT NULL,
  document_id BIGINT NOT NULL,
  chunk_order INT NOT NULL,
  chunk_text LONGTEXT NOT NULL,
  token_count INT NULL,
  qdrant_point_id VARCHAR(64) NULL UNIQUE,
  is_active TINYINT(1) NOT NULL DEFAULT 1,
  created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  UNIQUE KEY uk_version_chunk_order (version_id, chunk_order),
  CONSTRAINT fk_segments_version FOREIGN KEY (version_id) REFERENCES document_versions(version_id),
  CONSTRAINT fk_segments_document FOREIGN KEY (document_id) REFERENCES documents(document_id)
);

CREATE TABLE IF NOT EXISTS answer_citations (
  citation_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  answer_id BIGINT NOT NULL,
  document_id BIGINT NOT NULL,
  version_id BIGINT NOT NULL,
  segment_id BIGINT NULL,
  cite_order INT NOT NULL,
  score DECIMAL(8,4) NULL,
  snippet_text TEXT NOT NULL,
  UNIQUE KEY uk_answer_cite_order (answer_id, cite_order),
  CONSTRAINT fk_citation_answer FOREIGN KEY (answer_id) REFERENCES answers(answer_id),
  CONSTRAINT fk_citation_document FOREIGN KEY (document_id) REFERENCES documents(document_id),
  CONSTRAINT fk_citation_version FOREIGN KEY (version_id) REFERENCES document_versions(version_id),
  CONSTRAINT fk_citation_segment FOREIGN KEY (segment_id) REFERENCES document_segments(segment_id)
);

CREATE TABLE IF NOT EXISTS agent_runs (
  run_id BIGINT PRIMARY KEY AUTO_INCREMENT,
  agent_type VARCHAR(50) NOT NULL,
  trigger_type VARCHAR(30) NOT NULL,
  operator_user_id BIGINT NULL,
  document_id BIGINT NULL,
  version_id BIGINT NULL,
  question_id BIGINT NULL,
  answer_id BIGINT NULL,
  status VARCHAR(20) NOT NULL,
  input_text LONGTEXT NULL,
  output_text LONGTEXT NULL,
  meta_json JSON NULL,
  started_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
  finished_at DATETIME NULL,
  CONSTRAINT fk_agent_user FOREIGN KEY (operator_user_id) REFERENCES users(user_id),
  CONSTRAINT fk_agent_document FOREIGN KEY (document_id) REFERENCES documents(document_id),
  CONSTRAINT fk_agent_version FOREIGN KEY (version_id) REFERENCES document_versions(version_id),
  CONSTRAINT fk_agent_question FOREIGN KEY (question_id) REFERENCES questions(question_id),
  CONSTRAINT fk_agent_answer FOREIGN KEY (answer_id) REFERENCES answers(answer_id)
);

INSERT INTO roles (role_id, role_name, description)
VALUES (1, '系统管理员', '系统默认管理员')
ON DUPLICATE KEY UPDATE role_name = VALUES(role_name), description = VALUES(description);

INSERT INTO roles (role_id, role_name, description)
VALUES (2, '知识管理员', '知识维护与运营管理员')
ON DUPLICATE KEY UPDATE role_name = VALUES(role_name), description = VALUES(description);

INSERT INTO roles (role_id, role_name, description)
VALUES (3, '普通用户', '普通知识使用者')
ON DUPLICATE KEY UPDATE role_name = VALUES(role_name), description = VALUES(description);

INSERT INTO users (user_id, role_id, username, password_hash, email, department)
VALUES (1, 1, 'admin', 'BOOTSTRAP_REPLACED_ON_STARTUP', 'admin@example.com', 'IT')
ON DUPLICATE KEY UPDATE username = VALUES(username), password_hash = VALUES(password_hash), email = VALUES(email), department = VALUES(department);

INSERT INTO users (user_id, role_id, username, password_hash, email, department)
VALUES (2, 2, 'editor', 'BOOTSTRAP_REPLACED_ON_STARTUP', 'editor@example.com', '知识运营')
ON DUPLICATE KEY UPDATE username = VALUES(username), password_hash = VALUES(password_hash), email = VALUES(email), department = VALUES(department);
