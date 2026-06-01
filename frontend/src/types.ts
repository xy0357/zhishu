export interface DocumentListItem {
  document_id: number;
  title: string;
  summary: string;
  category_name: string;
  status: string;
  version_no: string;
  is_favorite: boolean;
  updated_at: string;
}

export interface DocumentVersion {
  version_id: number;
  version_no: string;
  title: string;
  content: string;
  summary: string;
  change_note: string;
  created_at: string;
}

export interface DocumentDetail {
  document_id: number;
  title: string;
  summary: string;
  content: string;
  category_name: string;
  status: string;
  version_no: string;
  is_favorite: boolean;
  tags: string[];
  versions: DocumentVersion[];
}

export interface CategoryItem {
  category_name: string;
  description: string;
  document_count: number;
}

export interface CategoryFormPayload {
  category_name: string;
  description: string;
}

export interface TagItem {
  tag_name: string;
  description: string;
  document_count: number;
}

export interface TagFormPayload {
  tag_name: string;
  description: string;
}

export interface FaqItem {
  faq_id: number;
  document_id: number;
  question: string;
  answer: string;
  created_at: string;
}

export interface FaqFormPayload {
  question: string;
  answer: string;
}

export interface DocumentFormPayload {
  title: string;
  summary: string;
  content: string;
  category_name: string;
  tags: string[];
  change_note: string;
}

export interface DashboardSummary {
  total_documents: number;
  published_documents: number;
  total_questions: number;
  total_agent_runs: number;
}

export interface Citation {
  cite_order: number;
  document_title: string;
  version_no: string;
  snippet_text: string;
  score: number;
}

export interface QaAnswer {
  answer_id: number;
  answer_text: string;
  confidence_score: number;
  citations: Citation[];
  created_at: string;
}

export interface QuestionHistoryItem {
  question_id: number;
  question_text: string;
  answer_preview: string;
  created_at: string;
}

export interface AgentRun {
  run_id: number;
  agent_type: string;
  trigger_type: string;
  status: string;
  input_text: string;
  output_text: string;
  started_at: string;
}

export interface UserItem {
  user_id: number;
  username: string;
  role_name: string;
  department: string;
  email: string;
}

export interface RoleItem {
  role_name: string;
  description: string;
  user_count: number;
}

export interface UserCreatePayload {
  username: string;
  role_name: string;
  department: string;
  email: string;
  password: string;
}

export interface UserUpdatePayload {
  username: string;
  role_name: string;
  department: string;
  email: string;
  password?: string;
}

export interface LoginPayload {
  username: string;
  password: string;
}

export interface AuthSession {
  access_token: string;
  token_type: string;
  user: UserItem;
}

export interface FavoriteDocumentItem {
  document_id: number;
  title: string;
  category_name: string;
  status: string;
  version_no: string;
  favorite_time: string;
}

export interface ReadRecordItem {
  read_id: number;
  document_id: number;
  title: string;
  category_name: string;
  status: string;
  version_no: string;
  read_time: string;
}

export interface FavoriteState {
  document_id: number;
  is_favorite: boolean;
}

export interface DeletedResource {
  resource_type: string;
  resource_key: string;
}

export interface ApiResponse<T> {
  success: boolean;
  message: string;
  data: T;
}
