import type { AxiosResponse } from 'axios';
import api from './api';

// --- Types ---

export interface UserListItem {
  user_id: string;
  username: string;
  email: string;
  display_name: string;
  department: string | null;
  job_title: string | null;
  is_active: boolean;
  last_login_at: string | null;
  created_at: string;
}

export interface User {
  user_id: string;
  username: string;
  email: string;
  display_name: string;
  first_name: string | null;
  last_name: string | null;
  department: string | null;
  job_title: string | null;
  entra_object_id: string | null;
  is_active: boolean;
  last_login_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface Role {
  role_id: string;
  role_code: string;
  role_name: string;
  description: string | null;
  is_system_role: boolean;
}

export interface UserWithRoles extends User {
  roles: Role[];
}

export interface PaginatedUsers {
  data: UserListItem[];
  total_count: number;
  page: number;
  page_size: number;
}

export interface ListUsersParams {
  query?: string;
  role_code?: string;
  is_active?: boolean;
  page?: number;
  page_size?: number;
}

export interface UpdateUserRequest {
  display_name?: string;
  department?: string;
  job_title?: string;
  is_active?: boolean;
}

export interface AssignRoleRequest {
  role_id: string;
}

// --- API functions ---

export const usersApi = {
  listUsers(params?: ListUsersParams): Promise<AxiosResponse<PaginatedUsers>> {
    return api.get('/users', { params });
  },

  getUser(userId: string): Promise<AxiosResponse<UserWithRoles>> {
    return api.get(`/users/${userId}`);
  },

  updateUser(userId: string, data: UpdateUserRequest): Promise<AxiosResponse<User>> {
    return api.put(`/users/${userId}`, data);
  },

  assignRole(userId: string, roleId: string): Promise<AxiosResponse<void>> {
    return api.post(`/users/${userId}/roles`, { role_id: roleId });
  },

  removeRole(userId: string, roleId: string): Promise<AxiosResponse<void>> {
    return api.delete(`/users/${userId}/roles/${roleId}`);
  },

  listRoles(): Promise<AxiosResponse<Role[]>> {
    return api.get('/roles');
  },
};
