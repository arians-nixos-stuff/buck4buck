terraform {
  required_providers {
    github = {
      source  = "integrations/github"
      version = "~> 6.0"
    }
  }
}

provider "github" {
  owner = "arians-nixos-stuff"
}

resource "github_repository" "buck4buck" {
  name = "buck4buck"

  allow_merge_commit     = false
  allow_squash_merge     = false
  allow_rebase_merge     = true
  allow_auto_merge       = true
  delete_branch_on_merge = true
}

data "github_app" "buildbuddy" {
  slug = "buildbuddy-io"
}

resource "github_repository_ruleset" "this" {
  name        = "default"
  repository  = github_repository.buck4buck.name
  target      = "branch"
  enforcement = "evaluate"

  conditions {
    ref_name {
      include = ["~DEFAULT_BRANCH"]
      exclude = []
    }
  }

  rules {
    deletion                = true
    non_fast_forward        = true
    required_linear_history = true
    required_signatures     = true

    merge_queue {
      merge_method = "REBASE" # default is MERGE but we want linear history :)

      grouping_strategy                 = "ALLGREEN"
      min_entries_to_merge_wait_minutes = 5
      min_entries_to_merge              = 1
      max_entries_to_merge              = 5
      max_entries_to_build              = 5
    }

    pull_request {
      allowed_merge_methods = ["rebase"]
    }

    required_status_checks {
      required_check {
        context        = "Test all targets"
        integration_id = data.github_app.buildbuddy.id
      }

      # The merge queue tests every PR against the current base branch, which is
      # what strict status checks would otherwise guarantee.
      strict_required_status_checks_policy = false
      do_not_enforce_on_create             = false
    }
  }
}

resource "github_repository_ruleset" "tags" {
  name        = "tags"
  repository  = github_repository.buck4buck.name
  target      = "tag"
  enforcement = "active"

  conditions {
    ref_name {
      include = ["~ALL"]
      exclude = []
    }
  }

  rules {
    update           = true
    deletion         = true
    non_fast_forward = true
  }
}

# The github provider has no resource for immutable releases yet:
# https://github.com/integrations/terraform-provider-github/issues/2746
resource "terraform_data" "immutable_releases" {
  input            = "repos/${github_repository.buck4buck.full_name}/immutable-releases"
  triggers_replace = [github_repository.buck4buck.repo_id]

  provisioner "local-exec" {
    command = "gh api -X PUT ${self.input}"
  }

  provisioner "local-exec" {
    when    = destroy
    command = "gh api -X DELETE ${self.input}"
  }
}
