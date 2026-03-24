import { describe, it, expect, vi, beforeEach } from 'vitest';

vi.mock('../api', () => ({
  default: {
    get: vi.fn(),
    post: vi.fn(),
    put: vi.fn(),
    delete: vi.fn(),
    interceptors: {
      request: { use: vi.fn() },
      response: { use: vi.fn() },
    },
  },
}));

import api from '../api';
import { usersApi } from '../usersApi';

describe('usersApi', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('getMyProfile calls GET /auth/me/profile', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await usersApi.getMyProfile();

    expect(api.get).toHaveBeenCalledWith('/auth/me/profile');
  });

  it('listUsers calls GET /users with params', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await usersApi.listUsers({ page: 1, page_size: 20 });

    expect(api.get).toHaveBeenCalledWith('/users', { params: { page: 1, page_size: 20 } });
  });

  it('lookupUsers calls GET /users/lookup', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] });

    await usersApi.lookupUsers();

    expect(api.get).toHaveBeenCalledWith('/users/lookup');
  });

  it('getUser calls GET /users/{id}', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await usersApi.getUser('user-123');

    expect(api.get).toHaveBeenCalledWith('/users/user-123');
  });

  it('updateUser calls PUT /users/{id}', async () => {
    (api.put as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await usersApi.updateUser('user-123', { display_name: 'New Name' });

    expect(api.put).toHaveBeenCalledWith('/users/user-123', { display_name: 'New Name' });
  });

  it('assignRole calls POST /users/{id}/roles', async () => {
    (api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await usersApi.assignRole('user-123', 'role-456');

    expect(api.post).toHaveBeenCalledWith('/users/user-123/roles', { role_id: 'role-456' });
  });

  it('removeRole calls DELETE /users/{id}/roles/{roleId}', async () => {
    (api.delete as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await usersApi.removeRole('user-123', 'role-456');

    expect(api.delete).toHaveBeenCalledWith('/users/user-123/roles/role-456');
  });

  it('confirmRoles calls POST /users/{id}/confirm-roles', async () => {
    (api.post as ReturnType<typeof vi.fn>).mockResolvedValue({ data: {} });

    await usersApi.confirmRoles('user-123');

    expect(api.post).toHaveBeenCalledWith('/users/user-123/confirm-roles');
  });

  it('listRoles calls GET /roles', async () => {
    (api.get as ReturnType<typeof vi.fn>).mockResolvedValue({ data: [] });

    await usersApi.listRoles();

    expect(api.get).toHaveBeenCalledWith('/roles');
  });
});
