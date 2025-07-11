#!/bin/bash

# Script to discover GitHub repositories
echo "🔍 Discovering your GitHub repositories..."

# Load environment variables
if [ -f .env ]; then
    source .env
    echo "✅ Loaded environment from .env file"
else
    echo "❌ No .env file found"
fi

if [ -z "$GITHUB_TOKEN" ]; then
    echo "❌ GITHUB_TOKEN is not set"
    echo "Please set it with: export GITHUB_TOKEN=\"your_token_here\""
    exit 1
fi

if [ -z "$GITHUB_USERNAME" ]; then
    echo "❌ GITHUB_USERNAME is not set"
    echo "Please set it with: export GITHUB_USERNAME=\"your_username\""
    exit 1
fi

echo "✅ Credentials found for user: $GITHUB_USERNAME"
echo ""

echo "📋 Your repositories:"
curl -s -H "Authorization: Bearer $GITHUB_TOKEN" \
     -H "Accept: application/vnd.github+json" \
     "https://api.github.com/user/repos?per_page=100" | \
     jq -r '.[] | "\(.owner.login)/\(.name)"' | \
     head -20

echo ""
echo "📋 Repositories you collaborate on:"
curl -s -H "Authorization: Bearer $GITHUB_TOKEN" \
     -H "Accept: application/vnd.github+json" \
     "https://api.github.com/user/repos?affiliation=collaborator&per_page=100" | \
     jq -r '.[] | "\(.owner.login)/\(.name)"' | \
     head -20

echo ""
echo "🏢 Your organizations:"
curl -s -H "Authorization: Bearer $GITHUB_TOKEN" \
     -H "Accept: application/vnd.github+json" \
     "https://api.github.com/user/orgs" | \
     jq -r '.[].login'
