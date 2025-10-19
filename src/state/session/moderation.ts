import {BSKY_LABELER_DID, BskyAgent} from '@atproto/api'

import {IS_TEST_USER} from '#/lib/constants'
import {getNoAppLabelers} from '../preferences/no-app-labelers'
import {configureAdditionalModerationAuthorities} from './additional-moderation-authorities'
import {readLabelers} from './agent-config'
import {type SessionAccount} from './types'

export function configureModerationForGuest() {
  switchToBskyAppLabeler()
  configureAdditionalModerationAuthorities()
}

export async function configureModerationForAccount(
  agent: BskyAgent,
  account: SessionAccount,
) {
  switchToBskyAppLabeler()
  if (IS_TEST_USER(account.handle)) {
    await trySwitchToTestAppLabeler(agent)
  }

  // The code below is actually relevant to production (and isn't global).
  const labelerDids = await readLabelers(account.did).catch(_ => {})
  if (labelerDids) {
    agent.configureLabelersHeader(labelerDids)
  } else {
    // If there are no headers in the storage, we'll not send them on the initial requests.
    // If we wanted to fix this, we could block on the preferences query here.
  }

  configureAdditionalModerationAuthorities()
}

function switchToBskyAppLabeler() {
  BskyAgent.configure({
    appLabelers: getNoAppLabelers() ? [] : [BSKY_LABELER_DID],
  })
}

async function trySwitchToTestAppLabeler(agent: BskyAgent) {
  const did = (
    await agent
      .resolveHandle({handle: 'mod-authority.test'})
      .catch(_ => undefined)
  )?.data.did
  if (did) {
    console.warn('USING TEST ENV MODERATION')
    BskyAgent.configure({appLabelers: [did]})
  }
}
