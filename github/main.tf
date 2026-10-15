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

resource "github_repository" "x" {
  name = "x"

  allow_merge_commit     = true
  allow_squash_merge     = false
  allow_rebase_merge     = true
  allow_auto_merge       = true
  delete_branch_on_merge = true
}

moved {
  from = github_repository.buck4buck
  to   = github_repository.x
}

data "github_app" "buildbuddy" {
  slug = "buildbuddy-io"
}

resource "github_repository_ruleset" "this" {
  name        = "default"
  repository  = github_repository.x.name
  target      = "branch"
  enforcement = "active"

  conditions {
    ref_name {
      include = ["~DEFAULT_BRANCH"]
      exclude = []
    }
  }

  # Imports of vendored and satellite repositories (third-party/README.md,
  # satellites/README.md) are merge commits of upstream history that must keep
  # their hashes, so they cannot go through the rebasing merge queue. Admins
  # bypass the queue and linear history and merge import pull requests with a
  # merge commit; everyone else can only get pull requests in through the queue.
  bypass_actors {
    actor_id    = 5 # the repository admin role
    actor_type  = "RepositoryRole"
    bypass_mode = "always"
  }

  rules {
    deletion                = true
    non_fast_forward        = true
    required_linear_history = true
    required_signatures     = false

    merge_queue {
      merge_method = "REBASE" # default is MERGE but we want linear history :)

      grouping_strategy                 = "ALLGREEN"
      min_entries_to_merge_wait_minutes = 5
      min_entries_to_merge              = 1
      max_entries_to_merge              = 5
      max_entries_to_build              = 5
    }

    pull_request {
      allowed_merge_methods = ["merge", "rebase"]
    }

    # required_status_checks {
    #   required_check {
    #     context        = "Test all targets"
    #     integration_id = data.github_app.buildbuddy.id
    #   }
    #
    #   # The merge queue tests every PR against the current base branch, which is
    #   # what strict status checks would otherwise guarantee.
    #   strict_required_status_checks_policy = false
    #   do_not_enforce_on_create             = false
    # }
  }
}

resource "github_repository_ruleset" "tags" {
  name        = "tags"
  repository  = github_repository.x.name
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
  input            = "repos/${github_repository.x.full_name}/immutable-releases"
  triggers_replace = [github_repository.x.repo_id]

  provisioner "local-exec" {
    command = "gh api -X PUT ${self.input}"
  }

  provisioner "local-exec" {
    when    = destroy
    command = "gh api -X DELETE ${self.input}"
  }
}
