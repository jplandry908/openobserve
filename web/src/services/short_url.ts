// Copyright 2023 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
import http from "./http";
const shortURL = {
  create: (org_identifier: string, url: string) => {
    return http().post(`/api/${org_identifier}/short`, {
      original_url: url,
    });
  },
  get: (org_identifier: string, id: string) => {
    return http().get(`/api/${org_identifier}/short/${id}?type=ui`);
  },
};
export default shortURL;
